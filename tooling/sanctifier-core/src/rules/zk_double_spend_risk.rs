use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, File, Item};

/// Z005 — Nullifier not checked before proof processing (double-spend risk).
///
/// A shielded contract that calls a verifier but never reads a nullifier set
/// from storage allows the same proof to be replayed for multiple withdrawals.
pub struct ZkDoubleSpendRiskRule;

impl ZkDoubleSpendRiskRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZkDoubleSpendRiskRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for ZkDoubleSpendRiskRule {
    fn name(&self) -> &str {
        "zk_double_spend_risk"
    }

    fn description(&self) -> &str {
        "Detects ZK verifier calls without a preceding nullifier-set check, enabling proof replay"
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
                        let body = quote::quote!(#f.block).to_string();

                        let calls_verifier = body.contains("verify_proof")
                            || body.contains("groth16_verify")
                            || body.contains("verify_groth16")
                            || body.contains("snark_verify");

                        if !calls_verifier {
                            continue;
                        }

                        let checks_nullifier = body.contains("nullifier")
                            || body.contains("Nullifier")
                            || body.contains("spent")
                            || body.contains("is_used")
                            || body.contains("is_spent");

                        if !checks_nullifier {
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Critical,
                                    format!(
                                        "Function '{}' verifies a ZK proof but never checks a nullifier set. \
                                        The same valid proof can be replayed to drain funds multiple times.",
                                        fn_name
                                    ),
                                    fn_name,
                                )
                                .with_suggestion(
                                    "Before processing a proof, load the nullifier from storage and assert it \
                                    has not been used. After successful verification, mark the nullifier as spent. \
                                    Use persistent storage so the nullifier set survives ledger compaction."
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
    fn flags_verifier_without_nullifier_check() {
        let rule = ZkDoubleSpendRiskRule::new();
        let source = r#"
            impl PrivatePool {
                pub fn withdraw(env: Env, proof: Vec<u8>, amount: i128) {
                    verify_proof(&env, &proof).expect("bad proof");
                    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(!v.is_empty(), "missing nullifier check must fire");
        assert!(v[0].message.contains("withdraw"));
    }

    #[test]
    fn no_violation_when_nullifier_checked() {
        let rule = ZkDoubleSpendRiskRule::new();
        let source = r#"
            impl PrivatePool {
                pub fn withdraw(env: Env, proof: Vec<u8>, nullifier: BytesN<32>, amount: i128) {
                    let is_spent: bool = env.storage().persistent()
                        .get(&nullifier).unwrap_or(false);
                    assert!(!is_spent, "nullifier already spent");
                    verify_proof(&env, &proof).expect("bad proof");
                    env.storage().persistent().set(&nullifier, &true);
                    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty(), "nullifier check present must not fire");
    }

    #[test]
    fn no_violation_for_non_verifier_function() {
        let rule = ZkDoubleSpendRiskRule::new();
        let source = r#"
            impl MyContract {
                pub fn deposit(env: Env, amount: i128) {
                    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
                }
            }
        "#;
        let v = rule.check(source);
        assert!(v.is_empty());
    }

    #[test]
    fn empty_source_is_safe() {
        let v = ZkDoubleSpendRiskRule::new().check("");
        assert!(v.is_empty());
    }
}
