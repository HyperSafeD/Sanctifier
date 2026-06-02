//! Rule S026 — taint propagation through tuple and struct destructures.
//!
//! Tracks user-controlled data (function parameters marked as tainted) through
//! variable assignments, including `let (a, b) = ...` (Pat::Tuple) and
//! `let Foo { x, y } = ...` (Pat::Struct) destructures.  Emits a finding when
//! a tainted value reaches a sensitive sink (storage write or external call)
//! without an intervening `require_auth` or explicit validation.

use super::{Rule, RuleViolation, Severity};
use std::collections::HashSet;
use syn::{parse_str, File, Item, Pat};

pub struct TaintPropagationRule;

impl TaintPropagationRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaintPropagationRule {
    fn default() -> Self {
        Self::new()
    }
}

// ── Internal state ─────────────────────────────────────────────────────────────

struct TaintState<'a> {
    fn_name: &'a str,
    /// Variables currently considered tainted.
    tainted: HashSet<String>,
    /// True once require_auth / require_auth_for_args has been seen.
    has_auth: bool,
    violations: Vec<TaintViolation>,
}

struct TaintViolation {
    fn_name: String,
    tainted_var: String,
    sink: String,
    line: usize,
}

// ── Rule impl ──────────────────────────────────────────────────────────────────

impl Rule for TaintPropagationRule {
    fn name(&self) -> &str {
        "taint_propagation"
    }

    fn description(&self) -> &str {
        "Tracks user-controlled data through tuple/struct destructures and flags \
         when tainted values reach storage or external-call sinks without auth"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file = match parse_str::<File>(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();

        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        if !matches!(f.vis, syn::Visibility::Public(_)) {
                            continue;
                        }

                        let fn_name = f.sig.ident.to_string();

                        // Seed taint from parameters (excluding `env: Env` and `self`)
                        let tainted = collect_param_names(&f.sig);
                        if tainted.is_empty() {
                            continue;
                        }

                        let mut state = TaintState {
                            fn_name: &fn_name,
                            tainted,
                            has_auth: false,
                            violations: Vec::new(),
                        };

                        scan_block(&f.block, &mut state);

                        for v in state.violations {
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Warning,
                                    format!(
                                        "Function '{}': tainted variable '{}' reaches '{}' \
                                         sink without prior require_auth",
                                        v.fn_name, v.tainted_var, v.sink
                                    ),
                                    format!("{}:{}", v.fn_name, v.line),
                                )
                                .with_suggestion(
                                    "Call require_auth() on any address parameter before using \
                                     user-controlled data in storage or external calls."
                                        .to_string(),
                                ),
                            );
                        }
                    }
                }
            }
        }

        violations
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ── Parameter extraction ───────────────────────────────────────────────────────

fn collect_param_names(sig: &syn::Signature) -> HashSet<String> {
    let mut names = HashSet::new();
    for arg in &sig.inputs {
        if let syn::FnArg::Typed(pt) = arg {
            // Skip Env parameters
            let ty_str = quote::quote!(#pt.ty).to_string();
            if ty_str.contains("Env") {
                continue;
            }
            collect_pat_idents(&pt.pat, &mut names);
        }
    }
    names
}

// ── Block / expression scanner ────────────────────────────────────────────────

fn scan_block(block: &syn::Block, state: &mut TaintState<'_>) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Local(local) => {
                // Check if the RHS expression contains a tainted variable
                if let Some(init) = &local.init {
                    let rhs_tainted = expr_is_tainted(&init.expr, &state.tainted);
                    // Also propagate auth checks from init expression
                    check_auth_in_expr(&init.expr, state);
                    if rhs_tainted {
                        // Propagate taint to all variables bound in the pattern
                        let mut new_tainted = HashSet::new();
                        collect_pat_idents(&local.pat, &mut new_tainted);
                        state.tainted.extend(new_tainted);
                    }
                }
            }
            syn::Stmt::Expr(e, _) => scan_expr(e, state),
            _ => {}
        }
    }
}

fn scan_expr(expr: &syn::Expr, state: &mut TaintState<'_>) {
    match expr {
        syn::Expr::MethodCall(mc) => {
            let method = mc.method.to_string();

            // Detect auth
            if method == "require_auth" || method == "require_auth_for_args" {
                state.has_auth = true;
            }

            // Detect sink: storage write or external call
            if is_storage_write(&method, &mc.receiver) || is_external_call(&method) {
                if !state.has_auth {
                    // Check if any argument uses a tainted variable
                    for arg in &mc.args {
                        if let Some(var) = first_tainted_ident(arg, &state.tainted) {
                            use syn::spanned::Spanned;
                            let line = mc.span().start().line;
                            state.violations.push(TaintViolation {
                                fn_name: state.fn_name.to_string(),
                                tainted_var: var,
                                sink: method.clone(),
                                line,
                            });
                        }
                    }
                }
            }

            scan_expr(&mc.receiver, state);
            for arg in &mc.args {
                scan_expr(arg, state);
            }
        }
        syn::Expr::Block(b) => scan_block(&b.block, state),
        syn::Expr::If(i) => {
            scan_expr(&i.cond, state);
            scan_block(&i.then_branch, state);
            if let Some((_, e)) = &i.else_branch {
                scan_expr(e, state);
            }
        }
        syn::Expr::Match(m) => {
            scan_expr(&m.expr, state);
            for arm in &m.arms {
                scan_expr(&arm.body, state);
            }
        }
        syn::Expr::Assign(a) => {
            scan_expr(&a.left, state);
            scan_expr(&a.right, state);
        }
        syn::Expr::Call(c) => {
            for arg in &c.args {
                scan_expr(arg, state);
            }
        }
        _ => {}
    }
}

fn check_auth_in_expr(expr: &syn::Expr, state: &mut TaintState<'_>) {
    if let syn::Expr::MethodCall(mc) = expr {
        let method = mc.method.to_string();
        if method == "require_auth" || method == "require_auth_for_args" {
            state.has_auth = true;
        }
    }
}

// ── Pattern helpers ────────────────────────────────────────────────────────────

/// Recursively collect all identifier names bound by a pattern.
/// Handles Pat::Ident, Pat::Tuple, and Pat::Struct — the key cases for #760.
fn collect_pat_idents(pat: &Pat, out: &mut HashSet<String>) {
    match pat {
        Pat::Ident(pi) => {
            out.insert(pi.ident.to_string());
        }
        // Pat::Tuple: let (a, b) = ...
        Pat::Tuple(pt) => {
            for elem in &pt.elems {
                collect_pat_idents(elem, out);
            }
        }
        // Pat::Struct: let Foo { x, y } = ...
        Pat::Struct(ps) => {
            for field in &ps.fields {
                collect_pat_idents(&field.pat, out);
            }
        }
        // Pat::TupleStruct: let Some(x) = ...
        Pat::TupleStruct(pts) => {
            for elem in &pts.elems {
                collect_pat_idents(elem, out);
            }
        }
        // Pat::Reference: let &x = ...
        Pat::Reference(pr) => collect_pat_idents(&pr.pat, out),
        _ => {}
    }
}

// ── Expression taint helpers ───────────────────────────────────────────────────

/// Returns true if the expression references any tainted variable.
fn expr_is_tainted(expr: &syn::Expr, tainted: &HashSet<String>) -> bool {
    first_tainted_ident(expr, tainted).is_some()
}

/// Returns the first tainted identifier name found in expr, or None.
fn first_tainted_ident(expr: &syn::Expr, tainted: &HashSet<String>) -> Option<String> {
    match expr {
        syn::Expr::Path(p) => {
            if let Some(seg) = p.path.segments.last() {
                let name = seg.ident.to_string();
                if tainted.contains(&name) {
                    return Some(name);
                }
            }
            None
        }
        syn::Expr::Reference(r) => first_tainted_ident(&r.expr, tainted),
        syn::Expr::MethodCall(mc) => {
            if let Some(v) = first_tainted_ident(&mc.receiver, tainted) {
                return Some(v);
            }
            for arg in &mc.args {
                if let Some(v) = first_tainted_ident(arg, tainted) {
                    return Some(v);
                }
            }
            None
        }
        syn::Expr::Call(c) => {
            for arg in &c.args {
                if let Some(v) = first_tainted_ident(arg, tainted) {
                    return Some(v);
                }
            }
            None
        }
        syn::Expr::Tuple(t) => {
            for elem in &t.elems {
                if let Some(v) = first_tainted_ident(elem, tainted) {
                    return Some(v);
                }
            }
            None
        }
        syn::Expr::Binary(b) => {
            first_tainted_ident(&b.left, tainted)
                .or_else(|| first_tainted_ident(&b.right, tainted))
        }
        _ => None,
    }
}

fn is_storage_write(method: &str, receiver: &syn::Expr) -> bool {
    if !matches!(method, "set" | "update" | "remove") {
        return false;
    }
    let s = quote::quote!(#receiver).to_string();
    s.contains("storage") || s.contains("persistent") || s.contains("temporary") || s.contains("instance")
}

fn is_external_call(method: &str) -> bool {
    matches!(method, "invoke_contract" | "try_invoke_contract" | "invoke_contract_check")
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> TaintPropagationRule {
        TaintPropagationRule::new()
    }

    #[test]
    fn detects_taint_through_tuple_destructure() {
        // Taint flows: user_data → (a, b) via tuple destructure → storage set with a
        let source = r#"
            impl MyContract {
                pub fn store_pair(env: Env, user_data: (Symbol, i128)) {
                    let (key, val) = user_data;
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(!v.is_empty(), "taint through tuple destructure must be flagged");
        assert!(v[0].message.contains("store_pair"));
    }

    #[test]
    fn detects_taint_through_struct_destructure() {
        let source = r#"
            impl MyContract {
                pub fn store_record(env: Env, record: MyRecord) {
                    let MyRecord { key, value } = record;
                    env.storage().persistent().set(&key, &value);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(!v.is_empty(), "taint through struct destructure must be flagged");
    }

    #[test]
    fn no_violation_when_require_auth_present() {
        let source = r#"
            impl MyContract {
                pub fn store_pair(env: Env, caller: Address, user_data: (Symbol, i128)) {
                    caller.require_auth();
                    let (key, val) = user_data;
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(v.is_empty(), "function with require_auth must not be flagged");
    }

    #[test]
    fn no_violation_for_private_function() {
        let source = r#"
            impl MyContract {
                fn internal_store(env: Env, key: Symbol, val: i128) {
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(v.is_empty(), "private functions are not entry points and must not be flagged");
    }

    #[test]
    fn direct_param_taint_to_storage() {
        // No destructure — param goes directly to storage key
        let source = r#"
            impl MyContract {
                pub fn bad_set(env: Env, key: Symbol, val: i128) {
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(!v.is_empty(), "direct taint to storage must be flagged");
    }

    #[test]
    fn empty_source_no_panic() {
        assert!(rule().check("").is_empty());
    }
}
