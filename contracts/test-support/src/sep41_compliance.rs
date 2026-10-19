//! Generic SEP-41 token-interface **compliance test harness**.
//!
//! # Why this module exists
//!
//! Historically every token contract in `contracts/` shipped its own ad-hoc
//! SEP-41 tests.  Those tests drifted: one contract checked `transfer_from`
//! authorization, another forgot it, a third never verified `burn_from` at
//! all.  There was no single definition of "this contract is a conformant
//! SEP-41 token".
//!
//! This harness fixes that.  It encodes the SEP-41 interface **once** and
//! exposes a generic entry point — [`assert_compliant`] — that any token
//! contract can be run through.  Point it at a contract's source and it
//! verifies that **all ten** SEP-41 functions are present, have the exact
//! specified signatures, and authorize the correct caller.  Any deviation
//! is a hard test failure.
//!
//! # Relationship to `sanctifier-core`
//!
//! The signature and authorization rules mirror Sanctifier's own `S012`
//! analysis pass (`sanctifier_core::sep41`).  They are intentionally
//! reimplemented here with **no dependency on `sanctifier-core`** so the
//! compliance suite stays a lightweight, self-contained conformance check
//! that does not drag the analyzer's heavy toolchain (Z3, etc.) into every
//! contract's test build.  If you change the SEP-41 spec in one place,
//! change it in both.
//!
//! # The SEP-41 interface
//!
//! | Function        | Signature                                                                 | Authorizes |
//! |-----------------|---------------------------------------------------------------------------|------------|
//! | `allowance`     | `(env, from: Address, spender: Address) -> i128`                          | —          |
//! | `approve`       | `(env, from: Address, spender: Address, amount: i128, expiration_ledger: u32)` | `from` |
//! | `balance`       | `(env, id: Address) -> i128`                                              | —          |
//! | `transfer`      | `(env, from: Address, to: MuxedAddress, amount: i128)`                    | `from`     |
//! | `transfer_from` | `(env, spender: Address, from: Address, to: Address, amount: i128)`       | `spender`  |
//! | `burn`          | `(env, from: Address, amount: i128)`                                      | `from`     |
//! | `burn_from`     | `(env, spender: Address, from: Address, amount: i128)`                    | `spender`  |
//! | `decimals`      | `(env) -> u32`                                                            | —          |
//! | `name`          | `(env) -> String`                                                         | —          |
//! | `symbol`        | `(env) -> String`                                                         | —          |
//!
//! # Example
//!
//! ```rust,ignore
//! use sanctifier_test_support::sep41_compliance::assert_compliant;
//!
//! #[test]
//! fn my_token_is_sep41_compliant() {
//!     assert_compliant("my-token", include_str!("../src/lib.rs"));
//! }
//! ```

use std::collections::HashSet;

use quote::quote;
use syn::visit::{self, Visit};
use syn::{parse_str, File, FnArg, Item, Pat, ReturnType, Type};

/// One required SEP-41 function and the contract it places on an implementer.
struct ExpectedFn {
    /// Function name, e.g. `"transfer"`.
    name: &'static str,
    /// Ordered `(param_name, canonical_type)` pairs, including the leading `env`.
    args: &'static [(&'static str, &'static str)],
    /// Canonical return type (`"()"` for no return).
    return_type: &'static str,
    /// Index (into `args`) of the parameter that MUST be authorized, if any.
    auth_param_index: Option<usize>,
}

/// The canonical SEP-41 interface — the single source of truth for the suite.
const SEP41_FUNCTIONS: [ExpectedFn; 10] = [
    ExpectedFn {
        name: "allowance",
        args: &[("env", "Env"), ("from", "Address"), ("spender", "Address")],
        return_type: "i128",
        auth_param_index: None,
    },
    ExpectedFn {
        name: "approve",
        args: &[
            ("env", "Env"),
            ("from", "Address"),
            ("spender", "Address"),
            ("amount", "i128"),
            ("expiration_ledger", "u32"),
        ],
        return_type: "()",
        auth_param_index: Some(1),
    },
    ExpectedFn {
        name: "balance",
        args: &[("env", "Env"), ("id", "Address")],
        return_type: "i128",
        auth_param_index: None,
    },
    ExpectedFn {
        name: "transfer",
        args: &[
            ("env", "Env"),
            ("from", "Address"),
            ("to", "MuxedAddress"),
            ("amount", "i128"),
        ],
        return_type: "()",
        auth_param_index: Some(1),
    },
    ExpectedFn {
        name: "transfer_from",
        args: &[
            ("env", "Env"),
            ("spender", "Address"),
            ("from", "Address"),
            ("to", "Address"),
            ("amount", "i128"),
        ],
        return_type: "()",
        auth_param_index: Some(1),
    },
    ExpectedFn {
        name: "burn",
        args: &[("env", "Env"), ("from", "Address"), ("amount", "i128")],
        return_type: "()",
        auth_param_index: Some(1),
    },
    ExpectedFn {
        name: "burn_from",
        args: &[
            ("env", "Env"),
            ("spender", "Address"),
            ("from", "Address"),
            ("amount", "i128"),
        ],
        return_type: "()",
        auth_param_index: Some(1),
    },
    ExpectedFn {
        name: "decimals",
        args: &[("env", "Env")],
        return_type: "u32",
        auth_param_index: None,
    },
    ExpectedFn {
        name: "name",
        args: &[("env", "Env")],
        return_type: "String",
        auth_param_index: None,
    },
    ExpectedFn {
        name: "symbol",
        args: &[("env", "Env")],
        return_type: "String",
        auth_param_index: None,
    },
];

/// The number of functions the SEP-41 interface requires.
pub const REQUIRED_FUNCTION_COUNT: usize = SEP41_FUNCTIONS.len();

/// The category of a single compliance failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueKind {
    /// A required function is absent.
    MissingFunction,
    /// A function exists but its signature does not match the specification.
    SignatureMismatch,
    /// A mutating function exists but does not authorize the required caller.
    AuthorizationMismatch,
}

/// A single SEP-41 compliance failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceIssue {
    /// Name of the offending function.
    pub function: String,
    /// What kind of deviation this is.
    pub kind: IssueKind,
    /// The signature the SEP-41 spec requires.
    pub expected: String,
    /// The signature actually found (absent for [`IssueKind::MissingFunction`]).
    pub actual: Option<String>,
    /// Human-readable explanation.
    pub message: String,
}

/// The outcome of checking one contract against the SEP-41 interface.
#[derive(Debug, Clone, Default)]
pub struct ComplianceReport {
    /// `true` only when every required function is present, correctly typed,
    /// and correctly authorized.
    pub compliant: bool,
    /// Names of the functions that passed every check.
    pub verified: Vec<String>,
    /// Every deviation found.
    pub issues: Vec<ComplianceIssue>,
}

impl ComplianceReport {
    /// Render a multi-line human-readable summary, used in assertion panics.
    pub fn render(&self, contract_name: &str) -> String {
        let mut out = format!(
            "SEP-41 compliance report for `{contract_name}`: {} ({} / {} functions verified)\n",
            if self.compliant {
                "COMPLIANT"
            } else {
                "NON-COMPLIANT"
            },
            self.verified.len(),
            REQUIRED_FUNCTION_COUNT,
        );
        for issue in &self.issues {
            out.push_str(&format!(
                "  ✗ [{:?}] {}\n      expected: {}\n",
                issue.kind, issue.message, issue.expected
            ));
            if let Some(actual) = &issue.actual {
                out.push_str(&format!("      found:    {actual}\n"));
            }
        }
        out
    }
}

/// A public method parsed out of a contract's `impl` block.
struct ParsedMethod {
    arg_types: Vec<String>,
    arg_names: Vec<Option<String>>,
    return_type: String,
    /// Indices (into the canonicalized arg list) that the body authorizes.
    authorized_params: HashSet<usize>,
}

impl ParsedMethod {
    fn signature(&self, name: &str) -> String {
        let args = self
            .arg_names
            .iter()
            .zip(self.arg_types.iter())
            .map(|(n, ty)| match n {
                Some(n) => format!("{n}: {ty}"),
                None => ty.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{name}({args}) -> {}", self.return_type)
    }
}

/// Check a contract's **source code** against the full SEP-41 interface.
///
/// Returns a [`ComplianceReport`].  Source that does not parse as Rust yields a
/// non-compliant report with a single synthetic issue rather than panicking, so
/// the harness degrades gracefully on malformed fixtures.
pub fn check(source: &str) -> ComplianceReport {
    let file = match parse_str::<File>(source) {
        Ok(file) => file,
        Err(err) => {
            return ComplianceReport {
                compliant: false,
                verified: Vec::new(),
                issues: vec![ComplianceIssue {
                    function: "<source>".to_string(),
                    kind: IssueKind::MissingFunction,
                    expected: "valid Rust source".to_string(),
                    actual: None,
                    message: format!("source did not parse as Rust: {err}"),
                }],
            };
        }
    };

    let methods = collect_public_methods(&file);

    let mut issues = Vec::new();
    let mut verified = Vec::new();

    for expected in &SEP41_FUNCTIONS {
        let expected_sig = render_expected(expected);
        match methods
            .iter()
            .find(|(name, _)| name.as_str() == expected.name)
        {
            None => issues.push(ComplianceIssue {
                function: expected.name.to_string(),
                kind: IssueKind::MissingFunction,
                expected: expected_sig,
                actual: None,
                message: format!("missing SEP-41 function `{}`", expected.name),
            }),
            Some((_, actual)) => {
                let expected_types: Vec<String> =
                    expected.args.iter().map(|(_, ty)| ty.to_string()).collect();

                if actual.arg_types != expected_types || actual.return_type != expected.return_type
                {
                    issues.push(ComplianceIssue {
                        function: expected.name.to_string(),
                        kind: IssueKind::SignatureMismatch,
                        expected: expected_sig,
                        actual: Some(actual.signature(expected.name)),
                        message: format!(
                            "function `{}` does not match the exact SEP-41 signature",
                            expected.name
                        ),
                    });
                    continue;
                }

                if let Some(auth_index) = expected.auth_param_index {
                    if !actual.authorized_params.contains(&auth_index) {
                        let authorizer = expected
                            .args
                            .get(auth_index)
                            .map(|(n, _)| *n)
                            .unwrap_or("caller");
                        issues.push(ComplianceIssue {
                            function: expected.name.to_string(),
                            kind: IssueKind::AuthorizationMismatch,
                            expected: expected_sig,
                            actual: Some(actual.signature(expected.name)),
                            message: format!(
                                "function `{}` must authorize `{}` (call `{}.require_auth()`)",
                                expected.name, authorizer, authorizer
                            ),
                        });
                        continue;
                    }
                }

                verified.push(expected.name.to_string());
            }
        }
    }

    verified.sort();
    ComplianceReport {
        compliant: issues.is_empty(),
        verified,
        issues,
    }
}

/// Assert that a contract's source is a fully compliant SEP-41 token.
///
/// This is the **generic entry point** of the suite: hand it any token
/// contract's source and it runs every SEP-41 check.  On any deviation it
/// panics with a full report, failing the test.
///
/// # Panics
///
/// Panics (failing the surrounding `#[test]`) if the contract is missing a
/// SEP-41 function, has a wrong signature, or fails to authorize a mutating
/// caller.
pub fn assert_compliant(contract_name: &str, source: &str) {
    let report = check(source);
    assert!(report.compliant, "{}", report.render(contract_name));
}

/// Assert that a contract is **not** SEP-41 compliant, and that at least one of
/// its deviations is of `expected_kind`.
///
/// Used by the suite's negative tests to prove that deviations are actually
/// caught — i.e. that the harness fails loudly when an implementer drifts from
/// the spec.
///
/// # Panics
///
/// Panics if the contract is compliant, or if it is non-compliant for reasons
/// that do not include `expected_kind`.
pub fn assert_deviates(contract_name: &str, source: &str, expected_kind: IssueKind) {
    let report = check(source);
    assert!(
        !report.compliant,
        "expected `{contract_name}` to deviate from SEP-41, but it was compliant"
    );
    assert!(
        report.issues.iter().any(|i| i.kind == expected_kind),
        "expected `{contract_name}` to have a {:?} issue, got: {}",
        expected_kind,
        report.render(contract_name)
    );
}

// ── syn plumbing (mirrors sanctifier_core::sep41) ───────────────────────────

fn collect_public_methods(file: &File) -> Vec<(String, ParsedMethod)> {
    let mut methods: Vec<(String, ParsedMethod)> = Vec::new();

    for item in &file.items {
        if let Item::Impl(item_impl) = item {
            for impl_item in &item_impl.items {
                if let syn::ImplItem::Fn(func) = impl_item {
                    if !matches!(func.vis, syn::Visibility::Public(_)) {
                        continue;
                    }
                    let name = func.sig.ident.to_string();
                    if methods.iter().any(|(n, _)| n == &name) {
                        continue; // keep the first definition, like the analyzer
                    }

                    let arg_types: Vec<String> = func
                        .sig
                        .inputs
                        .iter()
                        .filter_map(|input| match input {
                            FnArg::Typed(typed) => Some(canonical_type(&typed.ty)),
                            FnArg::Receiver(_) => None,
                        })
                        .collect();

                    let arg_names: Vec<Option<String>> = func
                        .sig
                        .inputs
                        .iter()
                        .filter_map(|input| match input {
                            FnArg::Typed(typed) => Some(pattern_name(&typed.pat)),
                            FnArg::Receiver(_) => None,
                        })
                        .collect();

                    let mut visitor = RequireAuthVisitor::default();
                    visitor.visit_block(&func.block);

                    let authorized_params = arg_names
                        .iter()
                        .enumerate()
                        .filter_map(|(index, name)| {
                            name.as_ref()
                                .filter(|name| visitor.authorized_names.contains(*name))
                                .map(|_| index)
                        })
                        .collect();

                    methods.push((
                        name,
                        ParsedMethod {
                            arg_types,
                            arg_names,
                            return_type: canonical_return_type(&func.sig.output),
                            authorized_params,
                        },
                    ));
                }
            }
        }
    }

    methods
}

fn render_expected(expected: &ExpectedFn) -> String {
    let args = expected
        .args
        .iter()
        .map(|(name, ty)| format!("{name}: {ty}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({}) -> {}", expected.name, args, expected.return_type)
}

fn canonical_return_type(output: &ReturnType) -> String {
    match output {
        ReturnType::Default => "()".to_string(),
        ReturnType::Type(_, ty) => canonical_type(ty),
    }
}

fn canonical_type(ty: &Type) -> String {
    match ty {
        Type::Group(group) => canonical_type(&group.elem),
        Type::Paren(paren) => canonical_type(&paren.elem),
        Type::Reference(reference) => format!("&{}", canonical_type(&reference.elem)),
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| simplify_tokens(&quote!(#ty).to_string())),
        Type::Tuple(tuple) if tuple.elems.is_empty() => "()".to_string(),
        _ => simplify_tokens(&quote!(#ty).to_string()),
    }
}

fn pattern_name(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(ident) => Some(ident.ident.to_string()),
        Pat::Reference(reference) => pattern_name(&reference.pat),
        Pat::Type(typed) => pattern_name(&typed.pat),
        Pat::Paren(paren) => pattern_name(&paren.pat),
        _ => None,
    }
}

fn simplify_tokens(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Default)]
struct RequireAuthVisitor {
    authorized_names: HashSet<String>,
}

impl<'ast> Visit<'ast> for RequireAuthVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let method_name = node.method.to_string();
        if method_name == "require_auth" || method_name == "require_auth_for_args" {
            if let Some(name) = expr_identifier(&node.receiver) {
                self.authorized_names.insert(name);
            }
            for arg in &node.args {
                if let Some(name) = expr_identifier(arg) {
                    self.authorized_names.insert(name);
                }
            }
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = &*node.func {
            if let Some(segment) = path.path.segments.last() {
                let ident = segment.ident.to_string();
                if ident == "require_auth" || ident == "require_auth_for_args" {
                    for arg in &node.args {
                        if let Some(name) = expr_identifier(arg) {
                            self.authorized_names.insert(name);
                        }
                    }
                }
            }
        }
        visit::visit_expr_call(self, node);
    }
}

fn expr_identifier(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        syn::Expr::Reference(reference) => expr_identifier(&reference.expr),
        syn::Expr::Paren(paren) => expr_identifier(&paren.expr),
        syn::Expr::Group(group) => expr_identifier(&group.expr),
        syn::Expr::Unary(unary) => expr_identifier(&unary.expr),
        syn::Expr::MethodCall(call) => expr_identifier(&call.receiver),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPLIANT: &str = r#"
        #![no_std]
        use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};
        #[contract]
        pub struct T;
        #[contractimpl]
        impl T {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) { from.require_auth(); }
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) { from.require_auth(); }
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) { spender.require_auth(); }
            pub fn burn(env: Env, from: Address, amount: i128) { from.require_auth(); }
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) { spender.require_auth(); }
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "T") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "T") }
        }
    "#;

    #[test]
    fn compliant_token_passes() {
        let report = check(COMPLIANT);
        assert!(report.compliant, "{}", report.render("inline"));
        assert_eq!(report.verified.len(), REQUIRED_FUNCTION_COUNT);
    }

    #[test]
    fn missing_function_is_flagged() {
        let src = COMPLIANT.replace("pub fn burn_from", "pub fn unrelated_burn_from");
        assert_deviates("missing", &src, IssueKind::MissingFunction);
    }

    #[test]
    fn wrong_signature_is_flagged() {
        // `amount: i64` instead of `i128`.
        let src = COMPLIANT.replace(
            "pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128)",
            "pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i64)",
        );
        assert_deviates("wrong-sig", &src, IssueKind::SignatureMismatch);
    }

    #[test]
    fn missing_auth_is_flagged() {
        let src = COMPLIANT.replace(
            "pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) { spender.require_auth(); }",
            "pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) { let _ = spender; }",
        );
        assert_deviates("no-auth", &src, IssueKind::AuthorizationMismatch);
    }

    #[test]
    fn non_token_contract_is_not_compliant() {
        let src = r#"
            pub struct Counter;
            impl Counter {
                pub fn increment(env: Env) -> u32 { 0 }
                pub fn get(env: Env) -> u32 { 0 }
            }
        "#;
        assert!(!check(src).compliant);
    }
}
