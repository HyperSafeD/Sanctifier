use sanctifier_core::smt::run_smt_latency_benchmark;
use std::fs;
use std::path::PathBuf;

#[test]
#[ignore = "benchmark utility test; run manually to profile SMT latency"]
fn benchmark_smt_solver_latency() {
    let report = run_smt_latency_benchmark(25);

    assert_eq!(report.strategies.len(), 3);
    assert!(report.strategies.iter().all(|s| s.runs == 25));
    assert!(report
        .strategies
        .iter()
        .all(|s| s.max_micros >= s.min_micros));

    let sorted = report.most_expensive_first();
    assert_eq!(sorted.len(), 3);
    for pair in sorted.windows(2) {
        assert!(pair[0].avg_micros >= pair[1].avg_micros);
    }

    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
    let output_path = PathBuf::from(target_dir).join("smt-latency-report.json");
    let json = serde_json::to_string_pretty(&report).expect("benchmark report should serialize");
    fs::write(&output_path, json).expect("failed to write SMT latency benchmark report");
}
