//! Integration tests for the Z3 backend module boundaries (S011).
//!
//! These tests verify that each sub-module of `sanctifier_core::smt` exposes
//! a stable, well-typed public boundary and that cross-module interactions
//! behave correctly.  They are black-box: every assertion is made through the
//! public re-exports on `sanctifier_core::smt`, never against internal symbols.
//!
//! # Running locally
//!
//! ```bash
//! cargo test --test smt_module_boundaries_test -p sanctifier-core
//! ```

#![cfg(feature = "smt")]

use sanctifier_core::smt::{
    parse_invariants, prove_fixed_point_mul_div_bounds,
    prove_fixed_point_mul_div_bounds_with_backend, run_smt_latency_benchmark, verify_invariants,
    FixedPointMulDivSpec, FixedPointProofError, SmtBackend, SmtConfig, SmtVerifier,
};

// ── 1. types module: data types are constructable and serialisable ─────────────

#[test]
fn smt_config_default_has_expected_timeout() {
    let cfg = SmtConfig::default();
    assert_eq!(cfg.timeout_ms, 10_000, "default timeout must be 10 000 ms");
}

#[test]
fn smt_backend_enum_variants_are_distinct() {
    assert_ne!(SmtBackend::Z3, SmtBackend::Cvc5);
}

#[test]
fn fixed_point_mul_div_spec_is_constructable() {
    let spec = FixedPointMulDivSpec {
        function_name: "test".to_string(),
        multiplicand_max: 100,
        multiplier_max: 100,
        divisor_min: 1,
        divisor_max: 10,
        result_max: None,
    };
    assert_eq!(spec.function_name, "test");
    assert_eq!(spec.divisor_min, 1);
}

// ── 2. invariants module: parse_invariants boundary ──────────────────────────

#[test]
fn parse_invariants_returns_empty_for_no_annotations() {
    let source = r#"
        impl Token {
            pub fn transfer(&self, amount: u64) {}
        }
    "#;
    let specs = parse_invariants(source);
    assert!(
        specs.is_empty(),
        "source with no #[invariant] attrs must return empty vec"
    );
}

#[test]
fn parse_invariants_extracts_multiple_annotations_from_different_functions() {
    let source = r#"
        impl Vault {
            #[invariant = "a + b <= u64::MAX"]
            pub fn deposit(&self, a: u64, b: u64) {}

            #[invariant = "x - y >= 0"]
            pub fn withdraw(&self, x: u64, y: u64) {}
        }
    "#;
    let specs = parse_invariants(source);
    assert_eq!(specs.len(), 2);

    let locations: Vec<&str> = specs.iter().map(|s| s.location.as_str()).collect();
    assert!(locations.contains(&"deposit"));
    assert!(locations.contains(&"withdraw"));
}

#[test]
fn parse_invariants_is_resilient_to_invalid_rust_source() {
    let specs = parse_invariants("not valid rust {{{{");
    assert!(
        specs.is_empty(),
        "parse error must return empty vec, not panic"
    );
}

// ── 3. invariants module: verify_invariants boundary ─────────────────────────

#[test]
fn verify_invariants_returns_finding_for_addition_overflow() {
    let source = r#"
        impl Calc {
            #[invariant = "a + b is safe"]
            pub fn add(&self, a: u64, b: u64) {}
        }
    "#;
    let findings = verify_invariants(source, &SmtConfig::default());
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0].counterexample.is_some(),
        "SAT result must include a counterexample"
    );
    assert!(
        !findings[0].is_timeout,
        "simple query must not time out with default config"
    );
}

#[test]
fn verify_invariants_returns_no_finding_for_non_arithmetic_invariant() {
    let source = r#"
        impl Token {
            #[invariant = "only_admin"]
            pub fn pause(&self) {}
        }
    "#;
    let findings = verify_invariants(source, &SmtConfig::default());
    assert!(
        findings.is_empty(),
        "non-arithmetic invariant must produce no finding"
    );
}

#[test]
fn verify_invariants_respects_timeout_config() {
    let source = r#"
        impl Vault {
            #[invariant = "a + b is safe"]
            pub fn credit(&self, a: u64, b: u64) {}
        }
    "#;
    // With a 1 ms timeout the solver may or may not finish.
    // Either way the invariant about timed-out findings must hold.
    let findings = verify_invariants(source, &SmtConfig { timeout_ms: 1 });
    for f in &findings {
        if f.is_timeout {
            assert!(
                f.counterexample.is_none(),
                "timed-out finding must not carry a counterexample"
            );
        }
    }
}

#[test]
fn verify_invariants_detects_subtraction_underflow() {
    let source = r#"
        impl Vault {
            #[invariant = "a - b does not underflow"]
            pub fn withdraw(&self, a: u64, b: u64) {}
        }
    "#;
    let findings = verify_invariants(source, &SmtConfig::default());
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0].counterexample.is_some(),
        "underflow counterexample must be present"
    );
}

// ── 4. backend module: SmtVerifier boundary ───────────────────────────────────

#[test]
fn smt_verifier_finds_u64_addition_overflow() {
    let cfg = z3::Config::new();
    let ctx = z3::Context::new(&cfg);
    let verifier = SmtVerifier::new(&ctx);

    let issue = verifier.verify_addition_overflow("add_fn", "add_fn:10");
    assert!(
        issue.is_some(),
        "unconstrained u64 addition is always overflowable — SmtVerifier must detect it"
    );
    let issue = issue.unwrap();
    assert_eq!(issue.function_name, "add_fn");
    assert_eq!(issue.location, "add_fn:10");
    assert!(
        issue.description.contains("overflow"),
        "issue description must mention overflow"
    );
}

// ── 5. backend module: fixed-point proof boundary ────────────────────────────

#[test]
fn fixed_point_proof_proves_safe_bounds() {
    let spec = FixedPointMulDivSpec {
        function_name: "price_calc".to_string(),
        multiplicand_max: 1_000_000_000,
        multiplier_max: 1_000_000_000,
        divisor_min: 1,
        divisor_max: 1_000_000,
        result_max: Some(u128::MAX),
    };
    let report = prove_fixed_point_mul_div_bounds(&spec).unwrap();
    assert!(report.proven_safe);
    assert_eq!(report.backend, SmtBackend::Z3);
    assert!(report.counterexample.is_none());
    assert!(!report.checked_properties.is_empty());
}

#[test]
fn fixed_point_proof_finds_unsafe_bounds() {
    let spec = FixedPointMulDivSpec {
        function_name: "overflow_calc".to_string(),
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
    assert!(!witness.multiplicand.is_empty());
}

#[test]
fn fixed_point_proof_rejects_zero_divisor_min() {
    let spec = FixedPointMulDivSpec {
        function_name: "bad_spec".to_string(),
        multiplicand_max: 100,
        multiplier_max: 100,
        divisor_min: 0,
        divisor_max: 10,
        result_max: None,
    };
    assert_eq!(
        prove_fixed_point_mul_div_bounds(&spec).unwrap_err(),
        FixedPointProofError::InvalidSpec("divisor_min must be greater than zero")
    );
}

#[test]
fn fixed_point_proof_with_backend_rejects_cvc5() {
    let spec = FixedPointMulDivSpec {
        function_name: "test".to_string(),
        multiplicand_max: 10,
        multiplier_max: 10,
        divisor_min: 1,
        divisor_max: 10,
        result_max: None,
    };
    assert_eq!(
        prove_fixed_point_mul_div_bounds_with_backend(SmtBackend::Cvc5, &spec).unwrap_err(),
        FixedPointProofError::UnsupportedBackend(SmtBackend::Cvc5)
    );
}

// ── 6. benchmark module: run_smt_latency_benchmark boundary ──────────────────

#[test]
fn latency_benchmark_returns_one_result_per_strategy() {
    let report = run_smt_latency_benchmark(3);
    assert_eq!(
        report.strategies.len(),
        3,
        "benchmark must return exactly one result per SmtProofStrategy variant"
    );
    assert_eq!(report.iterations_per_strategy, 3);
}

#[test]
fn latency_benchmark_statistics_are_internally_consistent() {
    let report = run_smt_latency_benchmark(5);
    for s in &report.strategies {
        assert!(
            s.min_micros <= s.avg_micros,
            "min must not exceed avg for {:?}",
            s.strategy
        );
        assert!(
            s.avg_micros <= s.max_micros,
            "avg must not exceed max for {:?}",
            s.strategy
        );
        assert_eq!(s.runs, 5);
    }
}

#[test]
fn latency_benchmark_most_expensive_first_is_sorted() {
    let report = run_smt_latency_benchmark(3);
    let sorted = report.most_expensive_first();
    assert_eq!(sorted.len(), 3);
    for pair in sorted.windows(2) {
        assert!(
            pair[0].avg_micros >= pair[1].avg_micros,
            "most_expensive_first must be sorted by descending avg_micros"
        );
    }
}

#[test]
fn latency_benchmark_report_is_json_serialisable() {
    let report = run_smt_latency_benchmark(2);
    let json = serde_json::to_string(&report)
        .expect("SmtLatencyBenchmarkReport must be JSON-serialisable");
    assert!(json.contains("strategies"));
    assert!(json.contains("iterations_per_strategy"));
    assert!(json.contains("timestamp"));
}

// ── 7. Cross-module: types flow correctly between invariants and backend ──────

#[test]
fn invariant_spec_location_matches_smt_finding_location() {
    let source = r#"
        impl Ledger {
            #[invariant = "credit + debit <= u64::MAX"]
            pub fn reconcile(&self, credit: u64, debit: u64) {}
        }
    "#;
    let specs = parse_invariants(source);
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].location, "reconcile");

    let findings = verify_invariants(source, &SmtConfig::default());
    assert_eq!(findings.len(), 1);
    // The location in the SmtFinding must match what parse_invariants extracted.
    assert_eq!(
        findings[0].location, specs[0].location,
        "SmtFinding.location must equal the InvariantSpec.location"
    );
}

#[test]
fn smt_finding_invariant_name_matches_original_expression() {
    let source = r#"
        impl Token {
            #[invariant = "balance + amount <= u64::MAX"]
            pub fn mint(&self, amount: u64) {}
        }
    "#;
    let findings = verify_invariants(source, &SmtConfig::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].invariant_name, "balance + amount <= u64::MAX",
        "SmtFinding.invariant_name must preserve the original expression verbatim"
    );
}
