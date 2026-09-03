//! Rule S025 — detect Persistent/Temporary storage writes without a TTL bump.
//!
//! Soroban storage entries expire. A contract that writes to `persistent()` or
//! `temporary()` storage but never calls `extend_ttl` on that entry will
//! silently lose data once the ledger TTL elapses.

use super::{Rule, RuleViolation, Severity};
use syn::spanned::Spanned;
use syn::{parse_str, File, Item};

pub struct MissingTtlBumpRule;

impl MissingTtlBumpRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MissingTtlBumpRule {
    fn default() -> Self {
        Self::new()
    }
}

// ── Internal state ─────────────────────────────────────────────────────────────

#[derive(Default)]
struct FnTtlState {
    /// Locations (fn_name, line) where persistent/temporary writes happen.
    writes: Vec<(String, usize)>,
    /// True if the function calls extend_ttl anywhere.
    has_extend_ttl: bool,
}

// ── Rule impl ──────────────────────────────────────────────────────────────────

impl Rule for MissingTtlBumpRule {
    fn name(&self) -> &str {
        "missing_ttl_bump"
    }

    fn description(&self) -> &str {
        "Detects writes to Persistent or Temporary storage without a corresponding \
         extend_ttl call — entries may silently expire and lose data"
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
                        let fn_name = f.sig.ident.to_string();
                        let mut state = FnTtlState::default();
                        scan_block(&f.block, &fn_name, &mut state);

                        if !state.has_extend_ttl {
                            for (name, line) in &state.writes {
                                violations.push(
                                    RuleViolation::new(
                                        self.name(),
                                        Severity::Warning,
                                        format!(
                                            "Function '{}' writes to Persistent/Temporary storage \
                                             but never calls extend_ttl — the entry may expire",
                                            name
                                        ),
                                        format!("{}:{}", name, line),
                                    )
                                    .with_suggestion(
                                        "Call env.storage().persistent().extend_ttl(&key, low, high) \
                                         (or temporary()) after each write to prevent the entry \
                                         from expiring. Example: \
                                         env.storage().instance().extend_ttl(1000, 5000);"
                                            .to_string(),
                                    ),
                                );
                            }
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

// ── AST helpers ────────────────────────────────────────────────────────────────

fn scan_block(block: &syn::Block, fn_name: &str, state: &mut FnTtlState) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    scan_expr(&init.expr, fn_name, state);
                }
            }
            syn::Stmt::Expr(e, _) => scan_expr(e, fn_name, state),
            _ => {}
        }
    }
}

fn scan_expr(expr: &syn::Expr, fn_name: &str, state: &mut FnTtlState) {
    match expr {
        syn::Expr::MethodCall(mc) => {
            let method = mc.method.to_string();

            if method == "extend_ttl" || method == "bump" {
                state.has_extend_ttl = true;
            }

            if is_persistent_or_temporary_write(&method, &mc.receiver) {
                let span = mc.span();
                state.writes.push((fn_name.to_string(), span.start().line));
            }

            scan_expr(&mc.receiver, fn_name, state);
            for arg in &mc.args {
                scan_expr(arg, fn_name, state);
            }
        }
        syn::Expr::Block(b) => scan_block(&b.block, fn_name, state),
        syn::Expr::If(i) => {
            scan_expr(&i.cond, fn_name, state);
            scan_block(&i.then_branch, fn_name, state);
            if let Some((_, else_expr)) = &i.else_branch {
                scan_expr(else_expr, fn_name, state);
            }
        }
        syn::Expr::Match(m) => {
            scan_expr(&m.expr, fn_name, state);
            for arm in &m.arms {
                scan_expr(&arm.body, fn_name, state);
            }
        }
        syn::Expr::Assign(a) => {
            scan_expr(&a.left, fn_name, state);
            scan_expr(&a.right, fn_name, state);
        }
        syn::Expr::Call(c) => {
            for arg in &c.args {
                scan_expr(arg, fn_name, state);
            }
        }
        _ => {}
    }
}

/// Returns true when `method` is a storage-mutating operation on a
/// `persistent()` or `temporary()` storage tier.
fn is_persistent_or_temporary_write(method: &str, receiver: &syn::Expr) -> bool {
    if !matches!(method, "set" | "update" | "remove") {
        return false;
    }
    let receiver_str = quote::quote!(#receiver).to_string();
    // Only flag persistent/temporary — instance storage has different TTL semantics
    receiver_str.contains("persistent") || receiver_str.contains("temporary")
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> MissingTtlBumpRule {
        MissingTtlBumpRule::new()
    }

    #[test]
    fn flags_persistent_write_without_extend_ttl() {
        let source = r#"
            impl MyContract {
                pub fn store(env: Env, key: Symbol, val: i128) {
                    env.storage().persistent().set(&key, &val);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            !v.is_empty(),
            "persistent write without TTL bump must be flagged"
        );
        assert!(v[0].message.contains("store"));
        assert_eq!(v[0].severity, Severity::Warning);
        assert!(v[0].suggestion.is_some());
    }

    #[test]
    fn flags_temporary_write_without_extend_ttl() {
        let source = r#"
            impl MyContract {
                pub fn cache(env: Env, key: Symbol, val: i128) {
                    env.storage().temporary().set(&key, &val);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(
            !v.is_empty(),
            "temporary write without TTL bump must be flagged"
        );
    }

    #[test]
    fn no_violation_when_extend_ttl_present() {
        let source = r#"
            impl MyContract {
                pub fn store_safe(env: Env, key: Symbol, val: i128) {
                    env.storage().persistent().set(&key, &val);
                    env.storage().persistent().extend_ttl(&key, 1000, 5000);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(v.is_empty(), "function with extend_ttl must not be flagged");
    }

    #[test]
    fn no_violation_when_instance_write_only() {
        // Instance storage has its own TTL semantics and is excluded from this rule
        let source = r#"
            impl MyContract {
                pub fn set_instance(env: Env) {
                    env.storage().instance().set(&symbol_short!("K"), &true);
                }
            }
        "#;
        let v = rule().check(source);
        assert!(v.is_empty(), "instance-only writes must not be flagged");
    }

    #[test]
    fn no_violation_for_read_only_function() {
        let source = r#"
            impl MyContract {
                pub fn get(env: Env, key: Symbol) -> i128 {
                    env.storage().persistent().get(&key).unwrap_or(0)
                }
            }
        "#;
        let v = rule().check(source);
        assert!(v.is_empty(), "read-only function must not be flagged");
    }

    #[test]
    fn empty_source_no_panic() {
        assert!(rule().check("").is_empty());
    }

    #[test]
    fn invalid_source_no_panic() {
        assert!(rule().check("not valid rust {{{").is_empty());
    }
}
