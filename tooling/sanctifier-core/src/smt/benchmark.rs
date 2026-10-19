//! SMT solver latency micro-benchmark.
//!
//! Runs each [`SmtProofStrategy`] for a configurable number of iterations and
//! collects min/max/avg/p95 latency statistics.  Results are returned as a
//! [`SmtLatencyBenchmarkReport`] that can be serialised to JSON and uploaded
//! as a CI artifact.
//!
//! This module has no production code path — it is only called from the
//! integration test `tests/smt_latency_benchmark.rs`.

use std::time::Instant;

use z3::ast::Int;
use z3::{Config, Context, SatResult, Solver};

use super::types::{SmtLatencyBenchmarkReport, SmtProofStrategy, SmtStrategyLatency};

// ── Public API ────────────────────────────────────────────────────────────────

/// Run a latency micro-benchmark for each [`SmtProofStrategy`].
///
/// Each strategy is run `iterations_per_strategy` times (minimum 1).
/// Returns an [`SmtLatencyBenchmarkReport`] with per-strategy statistics.
pub fn run_smt_latency_benchmark(iterations_per_strategy: usize) -> SmtLatencyBenchmarkReport {
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

// ── Internal: strategy runner ─────────────────────────────────────────────────

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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_returns_three_strategy_results() {
        let report = run_smt_latency_benchmark(3);
        assert_eq!(report.strategies.len(), 3);
        assert_eq!(report.iterations_per_strategy, 3);
    }

    #[test]
    fn benchmark_min_never_exceeds_max() {
        let report = run_smt_latency_benchmark(5);
        for s in &report.strategies {
            assert!(
                s.min_micros <= s.max_micros,
                "min ({}) must not exceed max ({}) for {:?}",
                s.min_micros,
                s.max_micros,
                s.strategy
            );
        }
    }

    #[test]
    fn benchmark_most_expensive_first_is_sorted_descending() {
        let report = run_smt_latency_benchmark(3);
        let sorted = report.most_expensive_first();
        for pair in sorted.windows(2) {
            assert!(
                pair[0].avg_micros >= pair[1].avg_micros,
                "most_expensive_first must be sorted by descending avg_micros"
            );
        }
    }

    #[test]
    fn benchmark_zero_iterations_treated_as_one() {
        let report = run_smt_latency_benchmark(0);
        assert_eq!(report.iterations_per_strategy, 1);
        for s in &report.strategies {
            assert_eq!(s.runs, 1);
        }
    }
}
