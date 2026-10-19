use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, visit::Visit, File};

/// Z013 — ZK proof verification result is not checked.
///
/// Calling `verify_proof(...)` and discarding the return value (no `.expect()`,
/// no `?`, no `match`, no `let` binding) means a failing proof is silently
/// ignored and state changes proceed anyway.
pub struct ZkVerificationResultIgnoredRule;

impl ZkVerificationResultIgnoredRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZkVerificationResultIgnoredRule {
    fn default() -> Self {
        Self::new()
    }
}

struct IgnoredResultVisitor {
    violations: Vec<String>,
    current_fn: String,
}

impl IgnoredResultVisitor {
    fn new() -> Self {
        Self {
            violations: Vec::new(),
            current_fn: String::new(),
        }
    }

    fn is_verifier_call(expr: &syn::Expr) -> bool {
        let s = quote::quote!(#expr).to_string();
        (s.contains("verify_proof")
            || s.contains("groth16_verify")
            || s.contains("verify_groth16")
            || s.contains("snark_verify"))
            && !s.contains("expect")
            && !s.contains("unwrap")
            && !s.contains("? ;")
            && !s.contains("?;")
    }
}

impl<'ast> Visit<'ast> for IgnoredResultVisitor {
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.current_fn = f.sig.ident.to_string();
        syn::visit::visit_impl_item_fn(self, f);
    }

    fn visit_stmt(&mut self, stmt: &'ast syn::Stmt) {
        // A bare expression statement (not a let binding) whose top-level call
        // is a verifier call means the result is dropped.
        if let syn::Stmt::Expr(expr, Some(_semi)) = stmt {
            if Self::is_verifier_call(expr) {
                self.violations.push(self.current_fn.clone());
            }
        }
        syn::visit::visit_stmt(self, stmt);
    }
}

impl Rule for ZkVerificationResultIgnoredRule {
    fn name(&self) -> &str {
        "zk_verification_result_ignored"
    }

    fn description(&self) -> &str {
        "Detects ZK proof verification calls whose return value is discarded, silently allowing invalid proofs"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file: File = match parse_str(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = IgnoredResultVisitor::new();
        visitor.visit_file(&file);

        visitor
            .violations
            .into_iter()
            .map(|fn_name| {
                RuleViolation::new(
                    self.name(),
                    Severity::Error,
                    format!(
                        "Function '{}' calls a ZK verifier but discards the result. \
                        A failing proof is silently ignored and execution continues.",
                        fn_name
                    ),
                    fn_name,
                )
                .with_suggestion(
                    "Always propagate the verifier result with `.expect(\"invalid proof\")`, \
                    the `?` operator, or an explicit match that panics or returns an error \
                    on verification failure."
                        .to_string(),
                )
            })
            .collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_bare_verifier_call() {
        let rule = ZkVerificationResultIgnoredRule::new();
        let source = r#"
            impl ShieldedTransfer {
                pub fn execute(env: Env, proof: Vec<u8>, amount: i128) {
                    verify_proof(&env, &proof);
                    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(!v.is_empty(), "ignored result must fire");
        assert!(v[0].message.contains("execute"));
    }

    #[test]
    fn no_violation_when_result_used_with_expect() {
        let rule = ZkVerificationResultIgnoredRule::new();
        let source = r#"
            impl ShieldedTransfer {
                pub fn execute(env: Env, proof: Vec<u8>, amount: i128) {
                    verify_proof(&env, &proof).expect("invalid proof");
                    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "checked result must not fire");
    }

    #[test]
    fn no_violation_when_result_bound_to_let() {
        let rule = ZkVerificationResultIgnoredRule::new();
        let source = r#"
            impl ShieldedTransfer {
                pub fn execute(env: Env, proof: Vec<u8>, amount: i128) {
                    let ok = verify_proof(&env, &proof);
                    assert!(ok, "proof failed");
                    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "let-bound result must not fire");
    }

    #[test]
    fn empty_source_is_safe() {
        let v = ZkVerificationResultIgnoredRule::new().check("");
        assert!(v.is_empty());
    }
}
