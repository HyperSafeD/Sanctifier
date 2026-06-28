//! Z3-based formal-verification primitives (S011).
//!
//! # Module layout
//!
//! | Sub-module | Responsibility |
//! |---|---|
//! | [`types`] | All shared data types and error enums |
//! | [`invariants`] | `#[invariant = "..."]` AST parsing and Z3 verification |
//! | [`backend`] | `SmtVerifier`, Z3 context wrapper, fixed-point proof dispatch |
//! | [`benchmark`] | Latency micro-benchmark for CI artifact generation |
//!
//! All items from every sub-module are re-exported at this level so that
//! existing call sites (`use sanctifier_core::smt::SmtFinding`, etc.) are
//! **unchanged** — this refactor has zero breaking surface.
//!
//! # Feature flag
//!
//! This entire module is gated behind `#[cfg(feature = "smt")]`.  Disable it
//! with `default-features = false` when targeting `wasm32-unknown-unknown`,
//! which has no native Z3 library.

mod backend;
mod benchmark;
mod invariants;
mod types;

// ── Public re-exports (zero-breaking-change surface) ─────────────────────────

// Types
pub use types::{
    FixedPointCounterexample, FixedPointMulDivSpec, FixedPointProofError, FixedPointProofReport,
    InvariantSpec, SmtBackend, SmtConfig, SmtFinding, SmtInvariantIssue, SmtLatencyBenchmarkReport,
    SmtProofStrategy, SmtStrategyLatency,
};

// Invariant verification (S011 entry-points)
pub use invariants::{parse_invariants, verify_invariants};

// Backend: SmtVerifier + fixed-point proofs
pub use backend::{
    prove_fixed_point_mul_div_bounds, prove_fixed_point_mul_div_bounds_with_backend, SmtVerifier,
};

// Benchmark
pub use benchmark::run_smt_latency_benchmark;
