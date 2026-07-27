use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, visit::Visit, Expr, File};

/// Z004 — Groth16/SNARK verifier call guarded by a skippable conditional.
///
/// If `verify_proof` (or equivalent) is called only inside an `if` branch that
/// also has an `else` path, an attacker may craft input that routes around the
/// verifier without a valid proof.
pub struct ZkVerifierSkippableRule;

impl ZkVerifierSkippableRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZkVerifierSkippableRule {
    fn default() -> Self {
        Self::new()
    }
}

struct SkippableVerifierVisitor {
    violations: Vec<String>,
    current_fn: String,
}

impl SkippableVerifierVisitor {
    fn new() -> Self {
        Self {
            violations: Vec::new(),
            current_fn: String::new(),
        }
    }

    fn expr_contains_verifier(expr: &Expr) -> bool {
        let s = quote::quote!(#expr).to_string();
        s.contains("verify_proof")
            || s.contains("groth16_verify")
            || s.contains("snark_verify")
            || s.contains("verify_groth16")
    }

    fn block_contains_verifier(block: &syn::Block) -> bool {
        let s = quote::quote!(#block).to_string();
        s.contains("verify_proof")
            || s.contains("groth16_verify")
            || s.contains("snark_verify")
            || s.contains("verify_groth16")
    }
}

impl<'ast> Visit<'ast> for SkippableVerifierVisitor {
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.current_fn = f.sig.ident.to_string();
        syn::visit::visit_impl_item_fn(self, f);
    }

    fn visit_expr_if(&mut self, expr_if: &'ast syn::ExprIf) {
        let then_has_verifier = Self::block_contains_verifier(&expr_if.then_branch);
        let has_else = expr_if.else_branch.is_some();

        if then_has_verifier && has_else {
            self.violations.push(self.current_fn.clone());
        }

        syn::visit::visit_expr_if(self, expr_if);
    }
}

impl Rule for ZkVerifierSkippableRule {
    fn name(&self) -> &str {
        "zk_verifier_skippable"
    }

    fn description(&self) -> &str {
        "Detects verifier calls inside if/else branches where the else path bypasses proof verification"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file: File = match parse_str(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut visitor = SkippableVerifierVisitor::new();
        visitor.visit_file(&file);

        visitor
            .violations
            .into_iter()
            .map(|fn_name| {
                RuleViolation::new(
                    self.name(),
                    Severity::Critical,
                    format!(
                        "Function '{}' calls the ZK verifier inside an if-branch that has an else \
                        path — an attacker can route execution around proof verification.",
                        fn_name
                    ),
                    fn_name,
                )
                .with_suggestion(
                    "Move the verifier call unconditionally before any branching logic. \
                    The proof must always be verified; use early-return on failure rather \
                    than an if/else that skips verification entirely."
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
    fn flags_verifier_in_skippable_if_else() {
        let rule = ZkVerifierSkippableRule::new();
        let source = r#"
            impl PrivateTransfer {
                pub fn transfer(env: Env, proof: Vec<u8>, use_zk: bool, amount: i128) {
                    if use_zk {
                        verify_proof(&env, &proof);
                    } else {
                        // bypass
                    }
                    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(!v.is_empty(), "skippable verifier must fire");
        assert!(v[0].message.contains("transfer"));
    }

    #[test]
    fn no_violation_when_verifier_unconditional() {
        let rule = ZkVerifierSkippableRule::new();
        let source = r#"
            impl PrivateTransfer {
                pub fn transfer(env: Env, proof: Vec<u8>, amount: i128) {
                    verify_proof(&env, &proof).expect("invalid proof");
                    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "unconditional verifier must not fire");
    }

    #[test]
    fn no_violation_for_if_without_else() {
        let rule = ZkVerifierSkippableRule::new();
        let source = r#"
            impl MyContract {
                pub fn maybe_verify(env: Env, proof: Vec<u8>, debug: bool) {
                    if debug {
                        verify_proof(&env, &proof);
                    }
                }
            }
        "#;
        // if without else is still suspicious but not the skippable pattern — no else branch.
        let v = rule.check(source);
        assert!(v.is_empty(), "if without else must not fire this rule");
    }

    #[test]
    fn empty_source_is_safe() {
        let v = ZkVerifierSkippableRule::new().check("");
        assert!(v.is_empty());
    }
}
