//! Shared types for the SMT formal-verification module.
//!
//! All public data types are defined here so that the other sub-modules
//! (`invariants`, `fixed_point`, `benchmark`) can import them without
//! creating circular dependencies.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Core finding type ─────────────────────────────────────────────────────────

/// An invariant issue proved by the Z3 SMT solver (finding code S011).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmtInvariantIssue {
    /// Function under verification.
    pub function_name: String,
    /// Human-readable description of the violation.
    pub description: String,
    /// Source location.
    pub location: String,
}

// ── Invariant verification types ──────────────────────────────────────────────

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

// ── Backend selector ──────────────────────────────────────────────────────────

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

// ── Circuit range-check types (Z007 deep-verify) ──────────────────────────────

/// Result of a circuit range-check verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircuitRangeCheckResult {
    /// Template name.
    pub template_name: String,
    /// Signals flagged as potentially under-constrained.
    pub flagged_signals: Vec<FlaggedSignal>,
}

/// A signal that may be under-constrained.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlaggedSignal {
    /// Signal name.
    pub signal_name: String,
    /// The SMT model (counterexample) when available.
    pub counterexample: Option<String>,
    /// Whether the solver timed out.
    pub is_timeout: bool,
}

// ── Fixed-point proof types ───────────────────────────────────────────────────

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

// ── Benchmark types ───────────────────────────────────────────────────────────

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
        sorted.sort_by_key(|b| std::cmp::Reverse(b.avg_micros));
        sorted
    }
}
