//! Integration / e2e tests for rule engine orchestration.
//!
//! These tests exercise the full orchestration pipeline end-to-end:
//! discovery order, filtering, deduplication, custom-rule execution,
//! and consistent outputs under repeated calls.
//!
//! They are intentionally black-box: every assertion is made through
//! the public `RuleRegistry` / `Analyzer` API, never against internal
//! implementation details.
//!
//! # Running locally
//!
//! ```bash
//! cargo test --test rule_engine_orchestration_test -p sanctifier-core
//! ```

use sanctifier_core::rules::{RuleRegistry, RuleViolation, Severity};
use sanctifier_core::{Analyzer, SanctifyConfig};

// ── Source fixtures ───────────────────────────────────────────────────────────

/// A contract with no detectable issues — expected to produce zero violations
/// across all built-in rules.
const CLEAN_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Address, Env};
    #[contract] pub struct Token;
    #[contractimpl] impl Token {
        pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
            from.require_auth();
        }
    }
"#;

/// A contract with a missing `require_auth` — triggers `auth_gap` (S001).
const AUTH_GAP_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};
    #[contract] pub struct Vault;
    #[contractimpl] impl Vault {
        pub fn withdraw(env: Env, recipient: Address, amount: i128) {
            env.storage().persistent().set(&recipient, &amount);
        }
    }
"#;

/// A contract with an unchecked addition — triggers `arithmetic_overflow` (S003).
const OVERFLOW_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Env};
    #[contract] pub struct Calc;
    #[contractimpl] impl Calc {
        pub fn add(_env: Env, a: u64, b: u64) -> u64 { a + b }
    }
"#;

/// A contract with a bare `panic!` — triggers `panic_detection` (S002).
const PANIC_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Env};
    #[contract] pub struct Risky;
    #[contractimpl] impl Risky {
        pub fn boom(_env: Env) { panic!("not implemented"); }
    }
"#;

/// A contract that triggers multiple distinct rules simultaneously.
const MULTI_ISSUE_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Address, Env};
    #[contract] pub struct Multi;
    #[contractimpl] impl Multi {
        pub fn dangerous(_env: Env, a: u64, b: u64, recipient: Address) -> u64 {
            // Missing require_auth  →  S001
            // Unchecked add         →  S003
            // Unwrap                →  S002 via panic_detection
            let x: Option<u64> = None;
            let _ = x.unwrap();
            a + b
        }
    }
"#;

// ── Helper ────────────────────────────────────────────────────────────────────

fn registry() -> RuleRegistry {
    RuleRegistry::with_default_rules()
}

fn rule_names_for(violations: &[RuleViolation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule_name.as_str()).collect()
}

// ── 1. Registry population ────────────────────────────────────────────────────

#[test]
fn default_registry_contains_all_expected_built_in_rules() {
    let reg = registry();
    let names = reg.available_rules();

    // Spot-check a representative subset of the 25 built-in rules.
    let required = [
        "auth_gap",
        "arithmetic_overflow",
        "panic_detection",
        "unhandled_result",
        "reentrancy",
        "ledger_size",
        "truncation_bounds",
        "unsafe_prng",
        "variable_shadowing",
        "instance_storage_misuse",
        "missing_state_event",
        "static_reentrancy",
    ];

    for r in &required {
        assert!(
            names.contains(r),
            "built-in rule '{r}' is missing from default registry"
        );
    }
}

#[test]
fn registry_has_no_duplicate_rule_names() {
    let reg = registry();
    let names = reg.available_rules();
    let mut seen = std::collections::HashSet::new();
    for n in &names {
        assert!(seen.insert(*n), "duplicate rule name: {n}");
    }
}

// ── 2. Empty-source / clean-source baseline ───────────────────────────────────

#[test]
fn empty_source_produces_no_violations() {
    let reg = registry();
    assert!(
        reg.run_all("").is_empty(),
        "empty source must produce no violations"
    );
}

#[test]
fn clean_contract_produces_no_violations() {
    let reg = registry();
    let violations = reg.run_all(CLEAN_CONTRACT);
    assert!(
        violations.is_empty(),
        "clean contract must produce no violations, got: {violations:?}"
    );
}

// ── 3. Per-rule firing ────────────────────────────────────────────────────────

#[test]
fn auth_gap_rule_fires_for_missing_require_auth() {
    let reg = registry();
    let violations = reg.run_by_name(AUTH_GAP_CONTRACT, "auth_gap");
    assert!(
        !violations.is_empty(),
        "auth_gap rule must fire for contract missing require_auth"
    );
    assert!(violations.iter().all(|v| v.rule_name == "auth_gap"));
}

#[test]
fn arithmetic_overflow_rule_fires_for_bare_addition() {
    let reg = registry();
    let violations = reg.run_by_name(OVERFLOW_CONTRACT, "arithmetic_overflow");
    assert!(
        !violations.is_empty(),
        "arithmetic_overflow must fire for bare `+`"
    );
    assert!(violations
        .iter()
        .all(|v| v.rule_name == "arithmetic_overflow"));
}

#[test]
fn panic_detection_rule_fires_for_bare_panic() {
    let reg = registry();
    let violations = reg.run_by_name(PANIC_CONTRACT, "panic_detection");
    assert!(
        !violations.is_empty(),
        "panic_detection must fire for bare panic!()"
    );
}

// ── 4. run_all fires multiple rules independently ─────────────────────────────

#[test]
fn run_all_fires_multiple_rules_on_multi_issue_contract() {
    let reg = registry();
    let violations = reg.run_all(MULTI_ISSUE_CONTRACT);
    let names = rule_names_for(&violations);

    // At minimum, two distinct rules must fire.
    let unique: std::collections::HashSet<_> = names.iter().copied().collect();
    assert!(
        unique.len() >= 2,
        "at least 2 distinct rules must fire on multi-issue contract, got: {unique:?}"
    );
}

#[test]
fn run_all_results_contain_all_per_rule_results() {
    // Each violation returned by run_all must also be returned by run_by_name
    // for the corresponding rule.
    let reg = registry();
    let all_violations = reg.run_all(MULTI_ISSUE_CONTRACT);
    for v in &all_violations {
        let per_rule = reg.run_by_name(MULTI_ISSUE_CONTRACT, &v.rule_name);
        let found = per_rule.iter().any(|p| {
            p.rule_name == v.rule_name && p.location == v.location && p.message == v.message
        });
        assert!(
            found,
            "violation from run_all not reproduced by run_by_name('{}') — location: {}",
            v.rule_name, v.location
        );
    }
}

// ── 5. Severity field integrity ────────────────────────────────────────────────

#[test]
fn every_violation_has_a_valid_severity() {
    let reg = registry();
    let violations = reg.run_all(MULTI_ISSUE_CONTRACT);
    // The point here is that no violation has an unrecognised/panicking severity.
    // Asserting all variants are representable.
    let valid = [
        Severity::Critical,
        Severity::Error,
        Severity::High,
        Severity::Warning,
        Severity::Info,
    ];
    for v in &violations {
        assert!(
            valid.contains(&v.severity),
            "violation from '{}' has unrecognised severity: {:?}",
            v.rule_name,
            v.severity
        );
    }
}

// ── 6. Location field integrity ───────────────────────────────────────────────

#[test]
fn every_violation_has_non_empty_location() {
    let reg = registry();
    let violations = reg.run_all(MULTI_ISSUE_CONTRACT);
    for v in &violations {
        assert!(
            !v.location.is_empty(),
            "violation from '{}' has an empty location field",
            v.rule_name
        );
    }
}

// ── 7. Determinism ────────────────────────────────────────────────────────────

#[test]
fn run_all_is_deterministic_across_repeated_calls() {
    let reg = registry();

    // Run 10 times, collect sorted (rule_name, location) pairs each time.
    let runs: Vec<Vec<(String, String)>> = (0..10)
        .map(|_| {
            let mut v: Vec<(String, String)> = reg
                .run_all(MULTI_ISSUE_CONTRACT)
                .into_iter()
                .map(|v| (v.rule_name, v.location))
                .collect();
            v.sort();
            v
        })
        .collect();

    let reference = &runs[0];
    for run in &runs[1..] {
        assert_eq!(
            reference, run,
            "run_all must return identical findings on repeated calls"
        );
    }
}

#[test]
fn run_by_name_is_deterministic_across_repeated_calls() {
    let reg = registry();

    let runs: Vec<Vec<String>> = (0..10)
        .map(|_| {
            let mut locs: Vec<String> = reg
                .run_by_name(AUTH_GAP_CONTRACT, "auth_gap")
                .into_iter()
                .map(|v| v.location)
                .collect();
            locs.sort();
            locs
        })
        .collect();

    let reference = &runs[0];
    for run in &runs[1..] {
        assert_eq!(
            reference, run,
            "run_by_name must return identical locations on repeated calls"
        );
    }
}

// ── 8. Analyzer integration — rule engine wired to Analyzer ──────────────────

#[test]
fn analyzer_run_rule_delegates_to_registry() {
    let a = Analyzer::new(SanctifyConfig::default());

    // run_rule("auth_gap") must agree with a direct scan_auth_gaps call.
    let via_registry = a.run_rule(AUTH_GAP_CONTRACT, "auth_gap");
    let via_scan = a.scan_auth_gaps(AUTH_GAP_CONTRACT);

    assert_eq!(
        via_registry.len(),
        via_scan.len(),
        "Analyzer::run_rule and scan_auth_gaps must agree on the finding count"
    );
}

#[test]
fn analyzer_run_rule_returns_empty_for_unknown_rule_name() {
    let a = Analyzer::new(SanctifyConfig::default());
    let violations = a.run_rule(AUTH_GAP_CONTRACT, "nonexistent_rule_xyz");
    assert!(
        violations.is_empty(),
        "unknown rule name must return an empty Vec, not panic"
    );
}

// ── 9. Custom regex rule execution ───────────────────────────────────────────

#[test]
fn custom_regex_rule_fires_on_matching_pattern() {
    let config = SanctifyConfig {
        rules: vec![sanctifier_core::CustomRule {
            name: "no_unsafe_test".to_string(),
            pattern: "unsafe\\s*\\{".to_string(),
            description: "No unsafe blocks".to_string(),
            severity: sanctifier_core::FindingSeverity::Medium,
        }],
        ..Default::default()
    };
    let a = Analyzer::new(config);
    let source = r#"
        pub fn danger() { unsafe { let _x = 1; } }
    "#;
    let matches = a.analyze_custom_rules(source);
    assert!(
        !matches.is_empty(),
        "custom regex rule must fire for `unsafe {{}}` blocks"
    );
    assert!(matches.iter().all(|m| m.rule_name == "no_unsafe_test"));
}

#[test]
fn custom_regex_rule_does_not_fire_on_non_matching_source() {
    let config = SanctifyConfig {
        rules: vec![sanctifier_core::CustomRule {
            name: "no_unsafe_test".to_string(),
            pattern: "unsafe\\s*\\{".to_string(),
            description: "No unsafe blocks".to_string(),
            severity: sanctifier_core::FindingSeverity::Medium,
        }],
        ..Default::default()
    };
    let a = Analyzer::new(config);
    let matches = a.analyze_custom_rules(CLEAN_CONTRACT);
    assert!(
        matches.is_empty(),
        "custom rule must not fire on clean contract"
    );
}

// ── 10. Registry is extensible without breaking existing rules ─────────────────

#[test]
fn registering_extra_rule_does_not_break_existing_rules() {
    use sanctifier_core::rules::{Rule, RuleViolation};

    struct NoOpRule;
    impl Rule for NoOpRule {
        fn name(&self) -> &str {
            "no_op_test_rule"
        }
        fn description(&self) -> &str {
            "A no-op rule for testing registry extensibility"
        }
        fn check(&self, _source: &str) -> Vec<RuleViolation> {
            vec![]
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    let mut reg = RuleRegistry::with_default_rules();
    let built_in_count = reg.available_rules().len();
    reg.register(NoOpRule);

    // Extra rule is visible.
    assert!(reg.available_rules().contains(&"no_op_test_rule"));
    // All original rules are still present.
    assert_eq!(reg.available_rules().len(), built_in_count + 1);

    // Existing rules still produce the same output.
    let violations = reg.run_by_name(AUTH_GAP_CONTRACT, "auth_gap");
    assert!(!violations.is_empty());
}

// ── 11. E2E: complete pipeline over multiple source files ─────────────────────

#[test]
fn end_to_end_scan_of_workspace_scale_inputs() {
    let reg = registry();

    let files = [
        ("contracts/clean.rs", CLEAN_CONTRACT),
        ("contracts/vault.rs", AUTH_GAP_CONTRACT),
        ("contracts/calc.rs", OVERFLOW_CONTRACT),
        ("contracts/risky.rs", PANIC_CONTRACT),
        ("contracts/multi.rs", MULTI_ISSUE_CONTRACT),
    ];

    let mut total_violations = 0usize;
    let mut files_with_violations = 0usize;

    for (name, src) in &files {
        let violations = reg.run_all(src);
        if !violations.is_empty() {
            files_with_violations += 1;
        }
        total_violations += violations.len();

        // Every violation must reference a known rule.
        let known_rules = reg.available_rules();
        for v in &violations {
            assert!(
                known_rules.contains(&v.rule_name.as_str()),
                "file {name}: violation references unknown rule '{}'",
                v.rule_name
            );
        }
    }

    // Clean contract should not contribute.
    let clean_violations = reg.run_all(CLEAN_CONTRACT).len();
    assert_eq!(clean_violations, 0);

    // At least 3 of the 5 files must have at least one finding.
    assert!(
        files_with_violations >= 3,
        "expected ≥3 files with violations, got {files_with_violations}"
    );
    assert!(
        total_violations >= 3,
        "expected ≥3 total violations across all files, got {total_violations}"
    );
}

// ── 12. Output format stability ───────────────────────────────────────────────

/// Verifies that the JSON-serialisable shape of `RuleViolation` remains stable.
/// Any change to the required fields must be intentional and version-bumped.
#[test]
fn rule_violation_serialises_with_required_fields() {
    let reg = registry();
    let violations = reg.run_by_name(AUTH_GAP_CONTRACT, "auth_gap");
    assert!(!violations.is_empty());

    let v = &violations[0];
    let json = serde_json::to_value(v).expect("RuleViolation must be JSON-serialisable");

    assert!(
        json.get("rule_name").is_some(),
        "serialised violation missing 'rule_name'"
    );
    assert!(
        json.get("severity").is_some(),
        "serialised violation missing 'severity'"
    );
    assert!(
        json.get("message").is_some(),
        "serialised violation missing 'message'"
    );
    assert!(
        json.get("location").is_some(),
        "serialised violation missing 'location'"
    );
}
