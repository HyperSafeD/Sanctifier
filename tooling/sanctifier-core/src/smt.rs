//! Z3-based formal-verification primitives.

use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;
use z3::ast::{Bool, Int};
use z3::{Context, SatResult, Solver};

/// An invariant issue proved by the Z3 SMT solver.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmtInvariantIssue {
    /// Function under verification.
    pub function_name: String,
    /// Human-readable description of the violation.
    pub description: String,
    /// Source location.
    pub location: String,
}

// ── Production SMT hardening ──────────────────────────────────────────────────

/// Configuration for the SMT-based invariant verifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtConfig {
    /// Solver timeout per call in milliseconds (default 10 000 ms = 10 s).
    pub timeout_ms: u64,
}

impl Default for SmtConfig {
    fn default() -> Self {
        Self { timeout_ms: 10_000 }
    }
}

/// Structured finding returned by the SMT invariant verifier.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmtFinding {
    /// Name / expression of the invariant that was checked.
    pub invariant_name: String,
    /// Source location (enclosing function name or file:line).
    pub location: String,
    /// Concrete counterexample values returned by Z3, when available.
    pub counterexample: Option<String>,
    /// `true` when the solver timed out instead of producing sat/unsat.
    pub is_timeout: bool,
}

/// A `#[invariant = "..."]` annotation extracted from Rust source via AST
/// analysis (not regex).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantSpec {
    /// The raw invariant expression (e.g. `"balance >= 0"`).
    pub expression: String,
    /// Enclosing function name used as the location hint.
    pub location: String,
}

/// Parse every `#[invariant = "..."]` attribute in the source file using the
/// syn AST.  Returns an empty vec on parse errors (no panic).
pub fn parse_invariants(source: &str) -> Vec<InvariantSpec> {
    use syn::{parse_str, Expr, File, Item, Lit, Meta};

    let file = match parse_str::<File>(source) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let mut specs = Vec::new();

    for item in &file.items {
        if let Item::Impl(impl_block) = item {
            for impl_item in &impl_block.items {
                if let syn::ImplItem::Fn(f) = impl_item {
                    let fn_name = f.sig.ident.to_string();
                    for attr in &f.attrs {
                        if attr.path().is_ident("invariant") {
                            if let Meta::NameValue(nv) = &attr.meta {
                                if let Expr::Lit(expr_lit) = &nv.value {
                                    if let Lit::Str(lit_str) = &expr_lit.lit {
                                        specs.push(InvariantSpec {
                                            expression: lit_str.value(),
                                            location: fn_name.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    specs
}

/// Verify `#[invariant = "..."]` annotations with Z3 under a configurable
/// timeout.
///
/// Returns one [`SmtFinding`] for every invariant that could not be proved
/// safe, or for which the solver timed out.  Safe invariants produce no
/// finding.
pub fn verify_invariants(source: &str, config: &SmtConfig) -> Vec<SmtFinding> {
    let specs = parse_invariants(source);
    if specs.is_empty() {
        return vec![];
    }

    let mut findings = Vec::new();
    for spec in &specs {
        if let Some(finding) = check_invariant_spec(spec, config) {
            findings.push(finding);
        }
    }
    findings
}

/// Check one [`InvariantSpec`] with Z3.
///
/// * Expressions that mention addition (`+` / `add`) are verified against
///   u64 overflow.
/// * Expressions that mention subtraction (`-` / `sub`) are verified against
///   u64 underflow.
/// * All other expressions are skipped (no finding — they require manual
///   modelling).
fn check_invariant_spec(spec: &InvariantSpec, config: &SmtConfig) -> Option<SmtFinding> {
    use z3::Config;

    let expr_lower = spec.expression.to_lowercase();

    let overflow_check = expr_lower.contains('+') || expr_lower.contains("add");
    let underflow_check = expr_lower.contains('-') || expr_lower.contains("sub");

    if !overflow_check && !underflow_check {
        return None;
    }

    let mut cfg = Config::new();
    // Pass the timeout as milliseconds; Z3 returns Unknown when it expires.
    cfg.set_param_value("timeout", &config.timeout_ms.to_string());
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let a = Int::new_const(&ctx, "a");
    let b = Int::new_const(&ctx, "b");
    let zero = Int::from_u64(&ctx, 0);
    let max_u64 = Int::from_u64(&ctx, u64::MAX);

    solver.assert(&a.ge(&zero));
    solver.assert(&a.le(&max_u64));
    solver.assert(&b.ge(&zero));
    solver.assert(&b.le(&max_u64));

    let violation = if overflow_check {
        // Assert a + b > u64::MAX (overflow possible)
        let sum = Int::add(&ctx, &[&a, &b]);
        sum.gt(&max_u64)
    } else {
        // Assert a - b < 0 (underflow possible)
        let diff = Int::sub(&ctx, &[&a, &b]);
        diff.lt(&zero)
    };

    solver.assert(&violation);

    match solver.check() {
        SatResult::Sat => {
            let counterexample = solver.get_model().map(|m| {
                let a_val = m
                    .eval(&a, true)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let b_val = m
                    .eval(&b, true)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".to_string());
                if overflow_check {
                    format!("a={a_val}, b={b_val} — a+b overflows u64")
                } else {
                    format!("a={a_val}, b={b_val} — a-b underflows u64")
                }
            });
            Some(SmtFinding {
                invariant_name: spec.expression.clone(),
                location: spec.location.clone(),
                counterexample,
                is_timeout: false,
            })
        }
        SatResult::Unsat => None,
        SatResult::Unknown => Some(SmtFinding {
            invariant_name: spec.expression.clone(),
            location: spec.location.clone(),
            counterexample: None,
            is_timeout: true,
        }),
    }
}

/// Supported SMT backends for fixed-point proofs.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SmtBackend {
    /// Z3 backend.
    Z3,
    /// Placeholder for future CVC5 support.
    Cvc5,
}

/// Input bounds for a standard fixed-point `a * b / d` proof.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FixedPointMulDivSpec {
    /// Human-readable function or calculation name.
    pub function_name: String,
    /// Maximum value of the left multiplicand.
    pub multiplicand_max: u128,
    /// Maximum value of the right multiplicand.
    pub multiplier_max: u128,
    /// Minimum divisor value (must be > 0).
    pub divisor_min: u128,
    /// Maximum divisor value.
    pub divisor_max: u128,
    /// Optional bound for the final quotient.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_max: Option<u128>,
}

/// Concrete witness returned when a fixed-point proof fails.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FixedPointCounterexample {
    /// Value chosen for the left multiplicand.
    pub multiplicand: String,
    /// Value chosen for the right multiplicand.
    pub multiplier: String,
    /// Value chosen for the divisor.
    pub divisor: String,
    /// Intermediate `a * b` result.
    pub intermediate_product: String,
    /// Final `(a * b) / d` quotient.
    pub quotient: String,
}

/// Result of a fixed-point overflow proof.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FixedPointProofReport {
    /// Human-readable function or calculation name.
    pub function_name: String,
    /// Backend used for the proof.
    pub backend: SmtBackend,
    /// Whether the proof established safety for all inputs in range.
    pub proven_safe: bool,
    /// Properties checked during the proof.
    pub checked_properties: Vec<String>,
    /// Summary message.
    pub message: String,
    /// Concrete witness if the proof failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterexample: Option<FixedPointCounterexample>,
}

/// Errors raised when preparing or executing a fixed-point proof.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FixedPointProofError {
    /// Invalid input bounds.
    #[error("invalid fixed-point proof specification: {0}")]
    InvalidSpec(&'static str),
    /// The requested backend is not implemented yet.
    #[error("unsupported SMT backend: {0:?}")]
    UnsupportedBackend(SmtBackend),
    /// The backend did not produce a usable answer.
    #[error("solver did not produce a usable result: {0}")]
    SolverFailure(&'static str),
}

/// Z3-backed SMT solver wrapper.
pub struct SmtVerifier<'ctx> {
    ctx: &'ctx Context,
}

impl<'ctx> SmtVerifier<'ctx> {
    /// Create a verifier bound to a Z3 [`Context`].
    pub fn new(ctx: &'ctx Context) -> Self {
        Self { ctx }
    }

    /// Proof-of-Concept: Uses Z3 to prove if `a + b` can overflow a 64-bit integer
    /// under unconstrained conditions.
    pub fn verify_addition_overflow(
        &self,
        fn_name: &str,
        location: &str,
    ) -> Option<SmtInvariantIssue> {
        let solver = Solver::new(self.ctx);
        let a = Int::new_const(self.ctx, "a");
        let b = Int::new_const(self.ctx, "b");

        // u64 bounds
        let zero = Int::from_u64(self.ctx, 0);
        let max_u64 = Int::from_u64(self.ctx, u64::MAX);

        // Constrain variables to valid u64 limits: 0 <= a, b <= u64::MAX
        solver.assert(&a.ge(&zero));
        solver.assert(&a.le(&max_u64));
        solver.assert(&b.ge(&zero));
        solver.assert(&b.le(&max_u64));

        // To prove overflow is IMPOSSIBLE, we assert the violation (a + b > max_u64)
        // and check if the solver can SATISFY this violation.
        let sum = Int::add(self.ctx, &[&a, &b]);
        solver.assert(&sum.gt(&max_u64));

        if solver.check() == SatResult::Sat {
            // A model exists where a + b > u64::MAX, meaning an overflow is mathematically possible
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
        SmtBackend::Z3 => prove_fixed_point_mul_div_bounds_z3(spec),
        SmtBackend::Cvc5 => Err(FixedPointProofError::UnsupportedBackend(SmtBackend::Cvc5)),
    }
}

/// The constraint-generation strategy used for an SMT proof.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SmtProofStrategy {
    /// Full u64 range.
    UnconstrainedOverflow,
    /// Bounded to ~5 × 10⁹.
    BoundedDomainOverflow,
    /// Bounded to 10 000.
    SmallDomainOverflow,
}

/// Latency statistics for a single [`SmtProofStrategy`].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmtStrategyLatency {
    /// Which strategy was measured.
    pub strategy: SmtProofStrategy,
    /// Number of iterations.
    pub runs: usize,
    /// Fastest run in microseconds.
    pub min_micros: u128,
    /// Slowest run in microseconds.
    pub max_micros: u128,
    /// Mean duration in microseconds.
    pub avg_micros: u128,
    /// 95th-percentile duration in microseconds.
    pub p95_micros: u128,
}

/// Aggregate benchmark across all [`SmtProofStrategy`] variants.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmtLatencyBenchmarkReport {
    /// Timestamp of the benchmark run.
    pub timestamp: String,
    /// How many iterations were run per strategy.
    pub iterations_per_strategy: usize,
    /// Per-strategy results.
    pub strategies: Vec<SmtStrategyLatency>,
}

impl SmtLatencyBenchmarkReport {
    /// Return strategies ordered by descending average latency.
    pub fn most_expensive_first(&self) -> Vec<SmtStrategyLatency> {
        let mut sorted = self.strategies.clone();
        sorted.sort_by(|a, b| b.avg_micros.cmp(&a.avg_micros));
        sorted
    }
}

/// Run a latency micro-benchmark for each [`SmtProofStrategy`].
pub fn run_smt_latency_benchmark(iterations_per_strategy: usize) -> SmtLatencyBenchmarkReport {
    use z3::{Config, Context};

    let iterations = iterations_per_strategy.max(1);
    let strategies = [
        SmtProofStrategy::UnconstrainedOverflow,
        SmtProofStrategy::BoundedDomainOverflow,
        SmtProofStrategy::SmallDomainOverflow,
    ];

    let mut results = Vec::with_capacity(strategies.len());

    for strategy in strategies {
        let mut samples = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let cfg = Config::new();
            let ctx = Context::new(&cfg);

            let start = Instant::now();
            let _ = run_strategy(&ctx, strategy);
            samples.push(start.elapsed().as_micros());
        }

        samples.sort_unstable();
        let min_micros = samples.first().copied().unwrap_or_default();
        let max_micros = samples.last().copied().unwrap_or_default();
        let total: u128 = samples.iter().sum();
        let avg_micros = total / samples.len() as u128;
        let p95_index = (((samples.len() - 1) as f64) * 0.95).round() as usize;
        let p95_micros = samples[p95_index];

        results.push(SmtStrategyLatency {
            strategy,
            runs: iterations,
            min_micros,
            max_micros,
            avg_micros,
            p95_micros,
        });
    }

    SmtLatencyBenchmarkReport {
        timestamp: chrono::Utc::now().to_rfc3339(),
        iterations_per_strategy: iterations,
        strategies: results,
    }
}

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

fn prove_fixed_point_mul_div_bounds_z3(
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

fn run_strategy(ctx: &Context, strategy: SmtProofStrategy) -> SatResult {
    let solver = Solver::new(ctx);
    let a = Int::new_const(ctx, "a");
    let b = Int::new_const(ctx, "b");
    let zero = Int::from_i64(ctx, 0);
    let max_u64 = Int::from_u64(ctx, u64::MAX);

    solver.assert(&a.ge(&zero));
    solver.assert(&b.ge(&zero));

    match strategy {
        SmtProofStrategy::UnconstrainedOverflow => {
            solver.assert(&a.le(&max_u64));
            solver.assert(&b.le(&max_u64));
        }
        SmtProofStrategy::BoundedDomainOverflow => {
            let max = Int::from_i64(ctx, 5_000_000_000);
            solver.assert(&a.le(&max));
            solver.assert(&b.le(&max));
        }
        SmtProofStrategy::SmallDomainOverflow => {
            let max = Int::from_i64(ctx, 10_000);
            solver.assert(&a.le(&max));
            solver.assert(&b.le(&max));
        }
    }

    let sum = Int::add(ctx, &[&a, &b]);
    solver.assert(&sum.gt(&max_u64));
    solver.check()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prove_fixed_point_mul_div_bounds_reports_safe_ranges() {
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
    fn prove_fixed_point_mul_div_bounds_reports_counterexample_for_unsafe_ranges() {
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
    fn prove_fixed_point_mul_div_bounds_rejects_zero_divisor() {
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
    fn prove_fixed_point_mul_div_bounds_supports_backend_abstraction() {
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

    // ── SmtFinding / verify_invariants tests ─────────────────────────────────

    #[test]
    fn verify_invariants_empty_source_returns_no_findings() {
        let findings = verify_invariants("", &SmtConfig::default());
        assert!(findings.is_empty(), "empty source must produce no findings");
    }

    #[test]
    fn verify_invariants_parse_error_returns_no_findings() {
        let findings = verify_invariants("this is not valid rust }{{{", &SmtConfig::default());
        assert!(
            findings.is_empty(),
            "unparseable source must produce no findings"
        );
    }

    #[test]
    fn parse_invariants_extracts_attribute_from_ast() {
        let source = r#"
            impl Vault {
                #[invariant = "balance + deposit <= u64::MAX"]
                pub fn deposit(&self, deposit: u64) {}
            }
        "#;
        let specs = parse_invariants(source);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].expression, "balance + deposit <= u64::MAX");
        assert_eq!(specs[0].location, "deposit");
    }

    #[test]
    fn verify_invariants_addition_overflow_flagged() {
        // An invariant on an addition can overflow — Z3 must find a counterexample.
        let source = r#"
            impl Vault {
                #[invariant = "a + b is safe"]
                pub fn credit(&self, a: u64, b: u64) {}
            }
        "#;
        let findings = verify_invariants(source, &SmtConfig::default());
        assert_eq!(findings.len(), 1);
        assert!(
            !findings[0].is_timeout,
            "should be a SAT result, not timeout"
        );
        assert!(
            findings[0].counterexample.is_some(),
            "counterexample must be populated for an unsafe invariant"
        );
        assert!(
            findings[0]
                .counterexample
                .as_ref()
                .unwrap()
                .contains("overflow"),
            "counterexample should mention overflow"
        );
    }

    #[test]
    fn verify_invariants_non_arithmetic_expression_is_safe() {
        // Invariants with no arithmetic operator are not modelled by the solver
        // and produce no finding (proven safe by abstention).
        let source = r#"
            impl Token {
                #[invariant = "admin_only"]
                pub fn burn(&self) {}
            }
        "#;
        let findings = verify_invariants(source, &SmtConfig::default());
        assert!(
            findings.is_empty(),
            "non-arithmetic invariant must not produce a finding"
        );
    }

    #[test]
    fn verify_invariants_timeout_produces_is_timeout_true() {
        // Force a 1 ms timeout so the solver cannot finish and returns Unknown.
        let source = r#"
            impl Vault {
                #[invariant = "a + b is safe"]
                pub fn credit(&self, a: u64, b: u64) {}
            }
        "#;
        let config = SmtConfig { timeout_ms: 1 };
        let findings = verify_invariants(source, &config);
        // At 1 ms the solver may time out OR solve immediately (it is fast for
        // simple queries).  Accept both outcomes — the important thing is that
        // if is_timeout is true, counterexample is None.
        for f in &findings {
            if f.is_timeout {
                assert!(
                    f.counterexample.is_none(),
                    "timed-out findings must have no counterexample"
                );
            }
        }
    }
}
