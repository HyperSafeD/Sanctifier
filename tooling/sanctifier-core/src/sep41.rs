//! SEP-41 token-interface compliance verification.

use quote::quote;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use syn::visit::{self, Visit};
use syn::{parse_str, File, FnArg, Item, Pat, ReturnType, Type};

/// The kind of SEP-41 compliance issue.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Sep41IssueKind {
    /// A required function is absent.
    MissingFunction,
    /// A function exists but its signature does not match the specification.
    SignatureMismatch,
    /// A function that should authorize a caller does not.
    AuthorizationMismatch,
}

/// A single SEP-41 compliance issue.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Sep41Issue {
    /// Name of the function with the issue.
    pub function_name: String,
    /// Category of the issue.
    pub kind: Sep41IssueKind,
    /// Source location.
    pub location: String,
    /// Human-readable description.
    pub message: String,
    /// The signature required by the specification.
    pub expected_signature: String,
    /// The actual signature found (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_signature: Option<String>,
}

/// Result of a full SEP-41 compliance check.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Sep41VerificationReport {
    /// Whether the contract looks like a SEP-41 token at all.
    pub candidate: bool,
    /// `true` if every required function is present and correct.
    pub compliant: bool,
    /// Functions that passed verification.
    pub verified_functions: Vec<String>,
    /// All detected issues.
    pub issues: Vec<Sep41Issue>,
}

struct ExpectedSep41Function {
    name: &'static str,
    args: &'static [(&'static str, &'static str)],
    return_type: &'static str,
    auth_param_index: Option<usize>,
}

#[derive(Debug, Clone)]
struct ParsedMethod {
    name: String,
    arg_types: Vec<String>,
    return_type: String,
    signature: String,
    authorized_params: HashSet<usize>,
}

const SEP41_FUNCTIONS: [ExpectedSep41Function; 10] = [
    ExpectedSep41Function {
        name: "allowance",
        args: &[("env", "Env"), ("from", "Address"), ("spender", "Address")],
        return_type: "i128",
        auth_param_index: None,
    },
    ExpectedSep41Function {
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
    ExpectedSep41Function {
        name: "balance",
        args: &[("env", "Env"), ("id", "Address")],
        return_type: "i128",
        auth_param_index: None,
    },
    ExpectedSep41Function {
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
    ExpectedSep41Function {
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
    ExpectedSep41Function {
        name: "burn",
        args: &[("env", "Env"), ("from", "Address"), ("amount", "i128")],
        return_type: "()",
        auth_param_index: Some(1),
    },
    ExpectedSep41Function {
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
    ExpectedSep41Function {
        name: "decimals",
        args: &[("env", "Env")],
        return_type: "u32",
        auth_param_index: None,
    },
    ExpectedSep41Function {
        name: "name",
        args: &[("env", "Env")],
        return_type: "String",
        auth_param_index: None,
    },
    ExpectedSep41Function {
        name: "symbol",
        args: &[("env", "Env")],
        return_type: "String",
        auth_param_index: None,
    },
];

/// Verify that `source` implements all 10 required SEP-41 functions.
pub fn verify(source: &str) -> Sep41VerificationReport {
    let file = match parse_str::<File>(source) {
        Ok(file) => file,
        Err(_) => return Sep41VerificationReport::default(),
    };

    let methods = collect_public_methods(&file);
    let candidate = looks_like_sep41_candidate(&methods);

    if !candidate {
        return Sep41VerificationReport::default();
    }

    let mut issues = Vec::new();
    let mut verified_functions = Vec::new();

    for expected in SEP41_FUNCTIONS {
        match methods.get(expected.name) {
            None => issues.push(Sep41Issue {
                function_name: expected.name.to_string(),
                kind: Sep41IssueKind::MissingFunction,
                location: expected.name.to_string(),
                message: format!("Missing SEP-41 function '{}'.", expected.name),
                expected_signature: render_expected_signature(&expected),
                actual_signature: None,
            }),
            Some(actual) => {
                let expected_arg_types: Vec<String> = expected
                    .args
                    .iter()
                    .map(|(_, ty)| (*ty).to_string())
                    .collect();

                if actual.arg_types != expected_arg_types
                    || actual.return_type != expected.return_type
                {
                    issues.push(Sep41Issue {
                        function_name: expected.name.to_string(),
                        kind: Sep41IssueKind::SignatureMismatch,
                        location: actual.name.clone(),
                        message: format!(
                            "Function '{}' does not match the exact SEP-41 signature.",
                            expected.name
                        ),
                        expected_signature: render_expected_signature(&expected),
                        actual_signature: Some(actual.signature.clone()),
                    });
                    continue;
                }

                if let Some(auth_index) = expected.auth_param_index {
                    if !actual.authorized_params.contains(&auth_index) {
                        let expected_authorizer = expected
                            .args
                            .get(auth_index)
                            .map(|(name, _)| *name)
                            .unwrap_or("authorizer");

                        issues.push(Sep41Issue {
                            function_name: expected.name.to_string(),
                            kind: Sep41IssueKind::AuthorizationMismatch,
                            location: actual.name.clone(),
                            message: format!(
                                "Function '{}' should authorize '{}' to match the SEP-41 interface.",
                                expected.name, expected_authorizer
                            ),
                            expected_signature: render_expected_signature(&expected),
                            actual_signature: Some(actual.signature.clone()),
                        });
                        continue;
                    }
                }

                verified_functions.push(expected.name.to_string());
            }
        }
    }

    verified_functions.sort();

    Sep41VerificationReport {
        candidate: true,
        compliant: issues.is_empty(),
        verified_functions,
        issues,
    }
}

fn collect_public_methods(file: &File) -> BTreeMap<String, ParsedMethod> {
    let mut methods = BTreeMap::new();

    for item in &file.items {
        if let Item::Impl(item_impl) = item {
            for impl_item in &item_impl.items {
                if let syn::ImplItem::Fn(func) = impl_item {
                    if !matches!(func.vis, syn::Visibility::Public(_)) {
                        continue;
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

                    let auth_visitor = {
                        let mut visitor = RequireAuthVisitor::default();
                        visitor.visit_block(&func.block);
                        visitor
                    };

                    let authorized_params = arg_names
                        .iter()
                        .enumerate()
                        .filter_map(|(index, name)| {
                            name.as_ref()
                                .filter(|name| auth_visitor.authorized_names.contains(*name))
                                .map(|_| index)
                        })
                        .collect();

                    let return_type = canonical_return_type(&func.sig.output);
                    let signature = render_actual_signature(
                        &func.sig.ident.to_string(),
                        &arg_names,
                        &arg_types,
                        &return_type,
                    );

                    let parsed = ParsedMethod {
                        name: func.sig.ident.to_string(),
                        arg_types,
                        return_type,
                        signature,
                        authorized_params,
                    };

                    methods.entry(parsed.name.clone()).or_insert(parsed);
                }
            }
        }
    }

    methods
}

fn looks_like_sep41_candidate(methods: &BTreeMap<String, ParsedMethod>) -> bool {
    let core_names = [
        "allowance",
        "approve",
        "balance",
        "transfer",
        "transfer_from",
        "burn",
        "burn_from",
    ];
    let metadata_names = ["decimals", "name", "symbol"];

    let core_count = core_names
        .iter()
        .filter(|name| methods.contains_key(**name))
        .count();
    let metadata_count = metadata_names
        .iter()
        .filter(|name| methods.contains_key(**name))
        .count();

    core_count >= 2 || (core_count >= 1 && metadata_count >= 2)
}

fn render_expected_signature(expected: &ExpectedSep41Function) -> String {
    let args = expected
        .args
        .iter()
        .map(|(name, ty)| format!("{name}: {ty}"))
        .collect::<Vec<_>>()
        .join(", ");

    format!("{}({}) -> {}", expected.name, args, expected.return_type)
}

fn render_actual_signature(
    name: &str,
    arg_names: &[Option<String>],
    arg_types: &[String],
    return_type: &str,
) -> String {
    let args = arg_names
        .iter()
        .zip(arg_types.iter())
        .map(|(name, ty)| match name {
            Some(name) => format!("{name}: {ty}"),
            None => ty.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("{name}({args}) -> {return_type}")
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
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_exact_sep41_interface() {
        let source = r#"
            use soroban_sdk::{Address, Env, MuxedAddress, String};

            #[contractimpl]
            impl Token {
                pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
                pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
                    from.require_auth();
                }
                pub fn balance(env: Env, id: Address) -> i128 { 0 }
                pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                    from.require_auth();
                }
                pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                    spender.require_auth();
                }
                pub fn burn(env: Env, from: Address, amount: i128) {
                    from.require_auth();
                }
                pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                    spender.require_auth();
                }
                pub fn decimals(env: Env) -> u32 { 7 }
                pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
                pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
            }
        "#;

        let report = verify(source);
        assert!(report.candidate);
        assert!(report.compliant);
        assert!(report.issues.is_empty());
        assert_eq!(report.verified_functions.len(), SEP41_FUNCTIONS.len());
    }

    #[test]
    fn reports_missing_sep41_functions() {
        let source = r#"
            use soroban_sdk::{Address, Env, String};

            #[contractimpl]
            impl Token {
                pub fn balance(env: Env, id: Address) -> i128 { 0 }
                pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {}
                pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            }
        "#;

        let report = verify(source);
        assert!(report.candidate);
        assert!(!report.compliant);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == Sep41IssueKind::MissingFunction
                && issue.function_name == "allowance"));
    }

    #[test]
    fn reports_signature_mismatches() {
        let source = r#"
            use soroban_sdk::{Address, Env, String};

            #[contractimpl]
            impl Token {
                pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
                pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
                    from.require_auth();
                }
                pub fn balance(env: Env, id: Address) -> i128 { 0 }
                pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                    from.require_auth();
                }
                pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                    spender.require_auth();
                }
                pub fn burn(env: Env, from: Address, amount: i128) {
                    from.require_auth();
                }
                pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                    spender.require_auth();
                }
                pub fn decimals(env: Env) -> u32 { 7 }
                pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
                pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
            }
        "#;

        let report = verify(source);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == Sep41IssueKind::SignatureMismatch
                && issue.function_name == "transfer"));
    }

    #[test]
    fn reports_authorization_mismatches() {
        let source = r#"
            use soroban_sdk::{Address, Env, MuxedAddress, String};

            #[contractimpl]
            impl Token {
                pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
                pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {}
                pub fn balance(env: Env, id: Address) -> i128 { 0 }
                pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                    from.require_auth();
                }
                pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                    spender.require_auth();
                }
                pub fn burn(env: Env, from: Address, amount: i128) {
                    from.require_auth();
                }
                pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                    spender.require_auth();
                }
                pub fn decimals(env: Env) -> u32 { 7 }
                pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
                pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
            }
        "#;

        let report = verify(source);
        assert!(report.issues.iter().any(|issue| {
            issue.kind == Sep41IssueKind::AuthorizationMismatch && issue.function_name == "approve"
        }));
    }

    #[test]
    fn ignores_non_token_contracts() {
        let source = r#"
            #[contractimpl]
            impl Counter {
                pub fn increment(env: Env) {}
                pub fn get(env: Env) -> u32 { 0 }
            }
        "#;

        let report = verify(source);
        assert!(!report.candidate);
        assert!(!report.compliant);
        assert!(report.issues.is_empty());
    }
}

impl Sep41Issue {
    /// Returns the severity level of this SEP-41 interface deviation.
    pub fn severity(&self) -> crate::finding_codes::FindingSeverity {
        crate::finding_codes::FindingSeverity::Critical
    }
}
