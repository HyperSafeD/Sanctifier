//! Z3 solver wrapper (`SmtVerifier`) and backend dispatch for fixed-point proofs.
//!
//! This module owns the Z3 `Context`-lifetime type [`SmtVerifier`] and the
//! top-level dispatcher [`prove_fixed_point_mul_div_bounds_with_backend`].
//! All Z3 API imports are confined here so the rest of the crate never touches
//! the `z3` crate directly.

use z3::ast::{Bool, Int};
use z3::{Context, SatResult, Solver};

use super::types::{
    FixedPointCounterexample, FixedPointMulDivSpec, FixedPointProofError, FixedPointProofReport,
    SmtBackend, SmtInvariantIssue,
};

// ── SmtVerifier ───────────────────────────────────────────────────────────────

/// Z3-backed SMT solver wrapper.
///
/// Bound to a single Z3 [`Context`] for its lifetime.  Create a new
/// `SmtVerifier` per verification session.
pub struct SmtVerifier<'ctx> {
    ctx: &'ctx Context,
}

impl<'ctx> SmtVerifier<'ctx> {
    /// Create a verifier bound to a Z3 [`Context`].
    pub fn new(ctx: &'ctx Context) -> Self {
        Self { ctx }
    }

    /// Uses Z3 to prove whether `a + b` can overflow a 64-bit unsigned integer
    /// under unconstrained inputs.
    ///
    /// Returns [`Some(SmtInvariantIssue)`] when overflow is reachable, `None`
    /// when the solver proves safety.
    pub fn verify_addition_overflow(
        &self,
        fn_name: &str,
        location: &str,
    ) -> Option<SmtInvariantIssue> {
        let solver = Solver::new(self.ctx);
        let a = Int::new_const(self.ctx, "a");
        let b = Int::new_const(self.ctx, "b");

        let zero = Int::from_u64(self.ctx, 0);
        let max_u64 = Int::from_u64(self.ctx, u64::MAX);

        solver.assert(&a.ge(&zero));
        solver.assert(&a.le(&max_u64));
        solver.assert(&b.ge(&zero));
        solver.assert(&b.le(&max_u64));

        // Assert the violation: a + b > u64::MAX.  SAT → overflow is reachable.
        let sum = Int::add(self.ctx, &[&a, &b]);
        solver.assert(&sum.gt(&max_u64));

        if solver.check() == SatResult::Sat {
            Some(SmtInvariantIssue {
                function_name: fn_name.to_string(),
                description: "SMT Solver (Z3) proved that this addition can overflow u64 bounds."
                    .to_string(),
                location: location.to_string(),
            })
        } else {
            None
        }
    }
}

// ── Fixed-point proof dispatch ────────────────────────────────────────────────

/// Prove that `a * b / d` cannot overflow `u128` within the provided bounds
/// using the default backend (Z3).
pub fn prove_fixed_point_mul_div_bounds(
    spec: &FixedPointMulDivSpec,
) -> Result<FixedPointProofReport, FixedPointProofError> {
    prove_fixed_point_mul_div_bounds_with_backend(SmtBackend::Z3, spec)
}

/// Prove that `a * b / d` cannot overflow `u128` within the provided bounds
/// using the selected SMT backend.
pub fn prove_fixed_point_mul_div_bounds_with_backend(
    backend: SmtBackend,
    spec: &FixedPointMulDivSpec,
) -> Result<FixedPointProofReport, FixedPointProofError> {
    validate_fixed_point_spec(spec)?;

    match backend {
        SmtBackend::Z3 => prove_fixed_point_z3(spec),
        SmtBackend::Cvc5 => Err(FixedPointProofError::UnsupportedBackend(SmtBackend::Cvc5)),
    }
}

// ── Internal: spec validation ─────────────────────────────────────────────────

fn validate_fixed_point_spec(spec: &FixedPointMulDivSpec) -> Result<(), FixedPointProofError> {
    if spec.divisor_min == 0 {
        return Err(FixedPointProofError::InvalidSpec(
            "divisor_min must be greater than zero",
        ));
    }
    if spec.divisor_max < spec.divisor_min {
        return Err(FixedPointProofError::InvalidSpec(
            "divisor_max must be greater than or equal to divisor_min",
        ));
    }
    Ok(())
}

// ── Internal: Z3 proof ────────────────────────────────────────────────────────

fn prove_fixed_point_z3(
    spec: &FixedPointMulDivSpec,
) -> Result<FixedPointProofReport, FixedPointProofError> {
    use z3::Config;

    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let multiplicand = Int::new_const(&ctx, "multiplicand");
    let multiplier = Int::new_const(&ctx, "multiplier");
    let divisor = Int::new_const(&ctx, "divisor");

    let zero = int_from_u128(&ctx, 0);
    let max_u128 = int_from_u128(&ctx, u128::MAX);
    let multiplicand_max = int_from_u128(&ctx, spec.multiplicand_max);
    let multiplier_max = int_from_u128(&ctx, spec.multiplier_max);
    let divisor_min = int_from_u128(&ctx, spec.divisor_min);
    let divisor_max = int_from_u128(&ctx, spec.divisor_max);

    solver.assert(&multiplicand.ge(&zero));
    solver.assert(&multiplicand.le(&multiplicand_max));
    solver.assert(&multiplier.ge(&zero));
    solver.assert(&multiplier.le(&multiplier_max));
    solver.assert(&divisor.ge(&divisor_min));
    solver.assert(&divisor.le(&divisor_max));

    let product = Int::mul(&ctx, &[&multiplicand, &multiplier]);
    let quotient = product.div(&divisor);
    let product_overflow = product.gt(&max_u128);

    let mut checked_properties = vec!["intermediate multiplication fits in u128".to_string()];

    let violation = if let Some(result_max) = spec.result_max {
        checked_properties.push(format!("final quotient <= {}", result_max));
        let quotient_overflow = quotient.gt(&int_from_u128(&ctx, result_max));
        Bool::or(&ctx, &[&product_overflow, &quotient_overflow])
    } else {
        product_overflow
    };

    solver.assert(&violation);

    match solver.check() {
        SatResult::Unsat => Ok(FixedPointProofReport {
            function_name: spec.function_name.clone(),
            backend: SmtBackend::Z3,
            proven_safe: true,
            checked_properties,
            message: "Z3 proved the fixed-point calculation stays within the configured bounds."
                .to_string(),
            counterexample: None,
        }),
        SatResult::Sat => {
            let model = solver
                .get_model()
                .ok_or(FixedPointProofError::SolverFailure(
                    "missing model for SAT result",
                ))?;

            let multiplicand_value =
                model
                    .eval(&multiplicand, true)
                    .ok_or(FixedPointProofError::SolverFailure(
                        "missing multiplicand witness",
                    ))?;
            let multiplier_value =
                model
                    .eval(&multiplier, true)
                    .ok_or(FixedPointProofError::SolverFailure(
                        "missing multiplier witness",
                    ))?;
            let divisor_value =
                model
                    .eval(&divisor, true)
                    .ok_or(FixedPointProofError::SolverFailure(
                        "missing divisor witness",
                    ))?;
            let product_value =
                model
                    .eval(&product, true)
                    .ok_or(FixedPointProofError::SolverFailure(
                        "missing product witness",
                    ))?;
            let quotient_value =
                model
                    .eval(&quotient, true)
                    .ok_or(FixedPointProofError::SolverFailure(
                        "missing quotient witness",
                    ))?;

            Ok(FixedPointProofReport {
                function_name: spec.function_name.clone(),
                backend: SmtBackend::Z3,
                proven_safe: false,
                checked_properties,
                message: "Z3 found a counterexample within the configured input ranges."
                    .to_string(),
                counterexample: Some(FixedPointCounterexample {
                    multiplicand: multiplicand_value.to_string(),
                    multiplier: multiplier_value.to_string(),
                    divisor: divisor_value.to_string(),
                    intermediate_product: product_value.to_string(),
                    quotient: quotient_value.to_string(),
                }),
            })
        }
        SatResult::Unknown => Err(FixedPointProofError::SolverFailure(
            "Z3 returned unknown for the requested fixed-point proof",
        )),
    }
}

fn int_from_u128<'ctx>(ctx: &'ctx Context, value: u128) -> Int<'ctx> {
    Int::from_str(ctx, &value.to_string()).expect("u128 literal should be a valid Z3 integer")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prove_fixed_point_reports_safe_ranges() {
        let spec = FixedPointMulDivSpec {
            function_name: "mul_div_floor".to_string(),
            multiplicand_max: 1_000_000_000_000_000_000,
            multiplier_max: 1_000_000_000_000_000_000,
            divisor_min: 1,
            divisor_max: 10_000_000,
            result_max: Some(u128::MAX),
        };
        let report = prove_fixed_point_mul_div_bounds(&spec).unwrap();
        assert!(report.proven_safe);
        assert!(report.counterexample.is_none());
    }

    #[test]
    fn prove_fixed_point_reports_counterexample_for_unsafe_ranges() {
        let spec = FixedPointMulDivSpec {
            function_name: "mul_div_floor".to_string(),
            multiplicand_max: u128::MAX,
            multiplier_max: 2,
            divisor_min: 1,
            divisor_max: 1,
            result_max: None,
        };
        let report = prove_fixed_point_mul_div_bounds(&spec).unwrap();
        assert!(!report.proven_safe);
        let witness = report.counterexample.unwrap();
        assert!(!witness.intermediate_product.is_empty());
    }

    #[test]
    fn prove_fixed_point_rejects_zero_divisor() {
        let spec = FixedPointMulDivSpec {
            function_name: "invalid".to_string(),
            multiplicand_max: 10,
            multiplier_max: 10,
            divisor_min: 0,
            divisor_max: 10,
            result_max: None,
        };
        let error = prove_fixed_point_mul_div_bounds(&spec).unwrap_err();
        assert_eq!(
            error,
            FixedPointProofError::InvalidSpec("divisor_min must be greater than zero")
        );
    }

    #[test]
    fn prove_fixed_point_rejects_inverted_divisor_range() {
        let spec = FixedPointMulDivSpec {
            function_name: "invalid".to_string(),
            multiplicand_max: 10,
            multiplier_max: 10,
            divisor_min: 10,
            divisor_max: 1,
            result_max: None,
        };
        let error = prove_fixed_point_mul_div_bounds(&spec).unwrap_err();
        assert_eq!(
            error,
            FixedPointProofError::InvalidSpec(
                "divisor_max must be greater than or equal to divisor_min"
            )
        );
    }

    #[test]
    fn prove_fixed_point_returns_unsupported_for_cvc5_backend() {
        let spec = FixedPointMulDivSpec {
            function_name: "mul_div_floor".to_string(),
            multiplicand_max: 10,
            multiplier_max: 10,
            divisor_min: 1,
            divisor_max: 10,
            result_max: None,
        };
        let error =
            prove_fixed_point_mul_div_bounds_with_backend(SmtBackend::Cvc5, &spec).unwrap_err();
        assert_eq!(
            error,
            FixedPointProofError::UnsupportedBackend(SmtBackend::Cvc5)
        );
    }

    #[test]
    fn smt_verifier_detects_addition_overflow() {
        let cfg = z3::Config::new();
        let ctx = Context::new(&cfg);
        let verifier = SmtVerifier::new(&ctx);
        let issue = verifier.verify_addition_overflow("test_fn", "test_fn:1");
        assert!(
            issue.is_some(),
            "unconstrained u64 addition must be flagged as potentially overflowing"
        );
        let issue = issue.unwrap();
        assert_eq!(issue.function_name, "test_fn");
        assert_eq!(issue.location, "test_fn:1");
    }
}
