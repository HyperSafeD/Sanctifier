# Benchmark Methodology

This document defines the methodology for benchmarking performance and solver latency in Sanctifier to ensure predictable outputs, reliable CI, and safe-by-default behavior as the project scales.

## Overview

Sanctifier uses two primary benchmarking vectors:
1.  **AST Analysis Latency**: Measures the speed of parsing Rust source and executing static analysis rules (tracked via `criterion`).
2.  **SMT Solver Latency**: Measures the time taken by the Z3 solver to prove or disprove invariants under various constraint strategies.

## SMT Latency Methodology

Solver performance is critical for CI/CD integration. We categorize SMT queries into three domain-driven strategies:

| Strategy | Domain Size | Focus | Typical Latency |
|----------|-------------|-------|-----------------|
| `UnconstrainedOverflow` | $2^{64}$ | Worst-case exhaustive proof | High |
| `BoundedDomainOverflow` | $\approx 5 \times 10^9$ | Real-world integer ranges | Medium |
| `SmallDomainOverflow` | $10,000$ | Unit-test style sanity checks | Low |

### Measuring Regression
- Benchmarks are run using `cargo test --test smt_latency_benchmark`.
- Reports are generated in `target/smt-latency-report.json`.
- **Target Stability**: Average latency for `SmallDomainOverflow` should remain $< 5ms$ to ensure developer productivity.

## AST Analysis Methodology

We benchmark core rule execution using `criterion` to prevent linear performance degradation on large contracts.

### Baseline Contract
We use `COMPLEX_CONTRACT_PAYLOAD` (a multi-function contract with complex storage and auth patterns) as our standard baseline.

### Key Metrics
- **Analyzer Initialization**: Must be near-instant ($< 100\mu s$).
- **Rule Execution**: Total execution time for a standard contract should not exceed $50ms$ per $1,000$ lines of code.

## CI/CD Integration

To maintain high velocity while ensuring performance:
1.  **Pre-commit**: Developers should run `scripts/check-benchmarks.sh` locally before pushing.
2.  **CI Pipeline**: SMT latency benchmarks run on every PR if `RUN_BENCHMARKS=1` is set.
3.  **Scheduled Runs**: Full Criterion benchmarks run weekly to track long-term performance trends.

## Safe-by-Default Defaults

- **Timeout**: All SMT calls default to a $10s$ timeout (`SmtConfig::default()`).
- **Precision**: By default, Sanctifier uses the `UnconstrainedOverflow` strategy for highest safety, falling back to bounded domains only when explicitly configured for performance-critical environments.
