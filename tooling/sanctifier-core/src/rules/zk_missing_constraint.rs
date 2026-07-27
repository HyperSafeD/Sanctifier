use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, File, Item};

/// Z001 — ZK circuit function has no constraint assertions.
///
/// Any function that accepts a `proof` or `verifying_key` parameter but never
/// calls a constraint-assertion helper (e.g. `assert_eq!`, `require`,
/// `verify_proof`, `check_constraint`) is likely unsound.
pub struct ZkMissingConstraintRule;

impl ZkMissingConstraintRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZkMissingConstraintRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for ZkMissingConstraintRule {
    fn name(&self) -> &str {
        "zk_missing_constraint"
    }

    fn description(&self) -> &str {
        "Detects ZK-related functions (proof/verifying_key parameters) that contain no constraint assertions"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file: File = match parse_str(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();

        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(f) = impl_item {
                        let fn_name = f.sig.ident.to_string();

                        let has_zk_param = f.sig.inputs.iter().any(|arg| {
                            let arg_str = quote::quote!(#arg).to_string();
                            arg_str.contains("proof")
                                || arg_str.contains("verifying_key")
                                || arg_str.contains("vk")
                                || arg_str.contains("Proof")
                        });

                        if !has_zk_param {
                            continue;
                        }

                        let body_str = quote::quote!(#f.block).to_string();
                        let has_constraint = body_str.contains("assert")
                            || body_str.contains("verify_proof")
                            || body_str.contains("check_constraint")
                            || body_str.contains("require")
                            || body_str.contains("verify");

                        if !has_constraint {
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Error,
                                    format!(
                                        "Function '{}' accepts a ZK proof parameter but has no constraint \
                                        assertions. Without constraint checks the proof is never verified \
                                        and the function is trivially exploitable.",
                                        fn_name
                                    ),
                                    fn_name.clone(),
                                )
                                .with_suggestion(
                                    "Call verify_proof() or assert the proof is valid before processing \
                                    any state changes. Every ZK-gated function must verify the proof \
                                    on-chain before trusting its public inputs."
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_proof_param_without_verify() {
        let rule = ZkMissingConstraintRule::new();
        let source = r#"
            impl ShieldedContract {
                pub fn withdraw(env: Env, proof: Vec<u8>, amount: i128) {
                    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(!v.is_empty(), "missing constraint must fire");
        assert!(v[0].message.contains("withdraw"));
    }

    #[test]
    fn no_violation_when_proof_verified() {
        let rule = ZkMissingConstraintRule::new();
        let source = r#"
            impl ShieldedContract {
                pub fn withdraw(env: Env, proof: Vec<u8>, amount: i128) {
                    verify_proof(&env, &proof).expect("invalid proof");
                    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "verified proof must not fire");
    }

    #[test]
    fn no_violation_for_non_zk_function() {
        let rule = ZkMissingConstraintRule::new();
        let source = r#"
            impl MyContract {
                pub fn transfer(env: Env, to: Address, amount: i128) {
                    env.storage().persistent().set(&to, &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "non-ZK function must not fire");
    }

    #[test]
    fn empty_source_is_safe() {
        let v = ZkMissingConstraintRule::new().check("");
        assert!(v.is_empty());
    }
}
