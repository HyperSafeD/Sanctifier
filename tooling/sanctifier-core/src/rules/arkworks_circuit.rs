//! Arkworks `ConstraintSynthesizer` circuit detection (Z007-adapted range-check analysis).
//!
//! Detects `impl ConstraintSynthesizer<_> for …` blocks in Rust source and applies
//! Z007-style range-check analysis to their `generate_constraints` methods using the
//! existing `syn`-based AST pipeline — no new parser is introduced.
//!
//! **What is flagged:**
//! - Witness allocations via `new_witness` / `new_input` / `new_constant` that are
//!   not followed by an `enforce_between`, `enforce_in_range`, `is_le`, or
//!   `range_check` call within the same function — a missing range check leaves the
//!   constraint system accepting out-of-range witnesses, which can enable soundness
//!   exploits in ZK proofs submitted to Soroban verification contracts.
//!
//! **What is NOT flagged:**
//! - Allocations explicitly followed by a range-constraint call.
//! - Non-`ConstraintSynthesizer` `impl` blocks (ordinary Soroban contracts are
//!   analysed by S-rules, not this rule).

use crate::rules::{Rule, RuleViolation, Severity};
use syn::{parse_str, File, ImplItem, Item, Type};

pub struct ArkworksCircuitRule;

impl ArkworksCircuitRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ArkworksCircuitRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for ArkworksCircuitRule {
    fn name(&self) -> &str {
        "arkworks_circuit_missing_range_check"
    }

    fn description(&self) -> &str {
        "Detects arkworks ConstraintSynthesizer implementations whose generate_constraints \
         function allocates witnesses without enforcing range bounds (Z007-adapted)"
    }

    fn check(&self, source: &str) -> Vec<RuleViolation> {
        let file: File = match parse_str(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut violations = Vec::new();

        for item in &file.items {
            if let Item::Impl(impl_block) = item {
                // Only analyse `impl ConstraintSynthesizer<_> for …` blocks.
                if !is_constraint_synthesizer_impl(impl_block) {
                    continue;
                }

                let self_ty = type_name(&impl_block.self_ty);

                for impl_item in &impl_block.items {
                    if let ImplItem::Fn(method) = impl_item {
                        let fn_name = method.sig.ident.to_string();
                        if fn_name != "generate_constraints" {
                            continue;
                        }

                        let body_tokens = format!("{:?}", method.block);
                        let alloc_count = count_witness_allocations(&body_tokens);
                        let range_count = count_range_constraints(&body_tokens);

                        if alloc_count > 0 && range_count == 0 {
                            violations.push(
                                RuleViolation::new(
                                    self.name(),
                                    Severity::Warning,
                                    format!(
                                        "`{}::generate_constraints` allocates {} witness variable(s) \
                                         but contains no range-constraint enforcement \
                                         (`enforce_between`, `enforce_in_range`, `is_le`, or `range_check`). \
                                         Without range checks the constraint system accepts \
                                         out-of-bounds witnesses, potentially breaking ZK soundness.",
                                        self_ty, alloc_count
                                    ),
                                    format!("{}::generate_constraints", self_ty),
                                )
                                .with_suggestion(
                                    "After each `new_witness` / `new_input` call, add a range \
                                     constraint via `FpVar::enforce_in_range`, \
                                     `UInt64::from_bits_le` with `enforce_between`, or an \
                                     equivalent arkworks gadget that bounds the witness value."
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

/// Returns `true` when the impl block's trait path contains `ConstraintSynthesizer`.
fn is_constraint_synthesizer_impl(impl_block: &syn::ItemImpl) -> bool {
    impl_block
        .trait_
        .as_ref()
        .map(|(_, path, _)| {
            path.segments
                .iter()
                .any(|seg| seg.ident == "ConstraintSynthesizer")
        })
        .unwrap_or(false)
}

/// Extract a human-readable name from a `Type`.
fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(p) => p
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        _ => "Unknown".to_string(),
    }
}

/// Count how many witness/input allocation calls appear in the debug representation.
fn count_witness_allocations(body: &str) -> usize {
    ["new_witness", "new_input", "new_constant"]
        .iter()
        .map(|kw| body.matches(kw).count())
        .sum()
}

/// Count how many range-constraint calls appear in the debug representation.
fn count_range_constraints(body: &str) -> usize {
    [
        "enforce_between",
        "enforce_in_range",
        "is_le",
        "range_check",
        "in_range",
    ]
    .iter()
    .map(|kw| body.matches(kw).count())
    .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── triggering fixtures ───────────────────────────────────────────────────

    #[test]
    fn flags_generate_constraints_without_range_check() {
        let rule = ArkworksCircuitRule::new();
        let source = r#"
            use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
            use ark_r1cs_std::fields::fp::FpVar;
            use ark_ff::PrimeField;

            struct RangeCircuit<F: PrimeField> {
                value: Option<F>,
            }

            impl<F: PrimeField> ConstraintSynthesizer<F> for RangeCircuit<F> {
                fn generate_constraints(
                    self,
                    cs: ConstraintSystemRef<F>,
                ) -> Result<(), SynthesisError> {
                    let val = FpVar::new_witness(cs.clone(), || {
                        self.value.ok_or(SynthesisError::AssignmentMissing)
                    })?;
                    // No range check here — should be flagged
                    Ok(())
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            !violations.is_empty(),
            "missing range check must be flagged"
        );
        assert!(violations[0].message.contains("range"));
        assert!(violations[0].message.contains("RangeCircuit"));
    }

    #[test]
    fn flags_multiple_witnesses_with_no_range_constraints() {
        let rule = ArkworksCircuitRule::new();
        let source = r#"
            use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
            use ark_r1cs_std::fields::fp::FpVar;

            struct MultiWitness { a: Option<u64>, b: Option<u64> }

            impl<F: ark_ff::PrimeField> ConstraintSynthesizer<F> for MultiWitness {
                fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
                    let _a = FpVar::new_witness(cs.clone(), || Ok(F::from(self.a.unwrap_or(0))))?;
                    let _b = FpVar::new_input(cs.clone(), || Ok(F::from(self.b.unwrap_or(0))))?;
                    Ok(())
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(!violations.is_empty());
        assert!(violations[0].message.contains("2 witness"));
    }

    // ── clean fixtures ────────────────────────────────────────────────────────

    #[test]
    fn no_violation_when_range_check_present() {
        let rule = ArkworksCircuitRule::new();
        let source = r#"
            use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
            use ark_r1cs_std::fields::fp::FpVar;

            struct SafeCircuit<F> { value: Option<F> }

            impl<F: ark_ff::PrimeField> ConstraintSynthesizer<F> for SafeCircuit<F> {
                fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
                    let val = FpVar::new_witness(cs.clone(), || self.value.ok_or(SynthesisError::AssignmentMissing))?;
                    val.enforce_in_range(0u64, u64::MAX)?;
                    Ok(())
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "range check present — should be clean"
        );
    }

    #[test]
    fn no_violation_when_is_le_used_as_range_guard() {
        let rule = ArkworksCircuitRule::new();
        let source = r#"
            struct BoundedCircuit { v: Option<u32> }
            impl<F: ark_ff::PrimeField> ConstraintSynthesizer<F> for BoundedCircuit {
                fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
                    let val = FpVar::new_witness(cs.clone(), || Ok(F::from(self.v.unwrap_or(0))))?;
                    let max = FpVar::new_constant(cs, F::from(u32::MAX as u64))?;
                    val.is_le(&max)?.enforce_equal(&Boolean::TRUE)?;
                    Ok(())
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(violations.is_empty(), "is_le range guard — should be clean");
    }

    #[test]
    fn ignores_non_constraint_synthesizer_impls() {
        let rule = ArkworksCircuitRule::new();
        let source = r#"
            struct MyContract;
            impl MyContract {
                fn generate_constraints(&self) -> Vec<u8> {
                    let val = self.new_witness();
                    val
                }
            }
        "#;
        let violations = rule.check(source);
        assert!(
            violations.is_empty(),
            "non-ConstraintSynthesizer impl must not be flagged"
        );
    }

    #[test]
    fn empty_source_produces_no_violations() {
        let rule = ArkworksCircuitRule::new();
        assert!(rule.check("").is_empty());
    }

    #[test]
    fn invalid_rust_produces_no_panic() {
        let rule = ArkworksCircuitRule::new();
        assert!(rule.check("impl {{ not valid rust @@@@").is_empty());
    }

    #[test]
    fn generate_constraints_with_no_allocations_is_clean() {
        let rule = ArkworksCircuitRule::new();
        let source = r#"
            struct EmptyCircuit;
            impl<F: ark_ff::PrimeField> ConstraintSynthesizer<F> for EmptyCircuit {
                fn generate_constraints(self, _cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
                    Ok(())
                }
            }
        "#;
        assert!(
            rule.check(source).is_empty(),
            "no allocations — nothing to check"
        );
    }
}
