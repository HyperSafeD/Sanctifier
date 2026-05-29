#![cfg(feature = "smt")]

use sanctifier_core::smt::run_smt_latency_benchmark;
use std::fs;
use std::path::PathBuf;

#[test]
fn benchmark_smt_solver_latency() {
    let run_benchmarks = std::env::var("RUN_BENCHMARKS").is_ok();
    if !run_benchmarks && cfg!(feature = "smt") {
        // Skip if not explicitly requested and not running on a custom feature set
        // But we want it to run in CI, so we'll check for CI env as well
        if std::env::var("GITHUB_ACTIONS").is_err() {
            return;
        }
    }

    let report = run_smt_latency_benchmark(25);

    assert_eq!(report.strategies.len(), 3);
    assert!(report.strategies.iter().all(|s| s.runs == 25));
    assert!(report
        .strategies
        .iter()
        .all(|s| s.max_micros >= s.min_micros));

    // DX: Ensure it's not impossibly fast (sanity check)
    assert!(report.strategies.iter().any(|s| s.avg_micros > 0));

    let sorted = report.most_expensive_first();
    assert_eq!(sorted.len(), 3);
    for pair in sorted.windows(2) {
        assert!(pair[0].avg_micros >= pair[1].avg_micros);
    }

    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
    let target_dir_path = PathBuf::from(target_dir);
    if !target_dir_path.exists() {
        fs::create_dir_all(&target_dir_path).expect("failed to create target directory");
    }
    let output_path = target_dir_path.join("smt-latency-report.json");
    let json = serde_json::to_string_pretty(&report).expect("benchmark report should serialize");
    fs::write(&output_path, json).expect("failed to write SMT latency benchmark report");

    // Print summary to stdout for CI visibility
    println!("SMT Latency Benchmark Summary:");
    for s in &report.strategies {
        println!(
            "  {:?}: avg {}µs, p95 {}µs",
            s.strategy, s.avg_micros, s.p95_micros
        );
    }
}
