//! Fixture-backed regression tests for the `reentrancy`, `raw_invoke_contract`
//! and `panic_detection` detector rules.
//!
//! Each rule is exercised end to end: a contract-shaped fixture from
//! `tests/fixtures/` is run through the default [`RuleRegistry`], the same
//! dispatch path the CLI uses, rather than through the rule's own `check`.
//! The fixture must make the rule under test fire, nothing unexpected may fire
//! alongside it, and the result is snapshotted in `tests/snapshots/`. If a
//! refactor of `rules/mod.rs` stops invoking one of these rules, its test here
//! fails.
//!
//! Regenerate snapshots after an intentional change:
//!   INSTA_UPDATE=always cargo test -p sanctifier-core --test rule_fixture_regressions_security_test

use insta::with_settings;
use sanctifier_core::rules::{RuleRegistry, RuleViolation};
use std::path::PathBuf;

fn read_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// Same normalisation as `sarif_snapshots.rs`: drop the `:line` suffix of the
/// location so snapshots do not shift when a fixture is reformatted.
fn violations_json(violations: &[RuleViolation]) -> serde_json::Value {
    let mut value = serde_json::to_value(violations).unwrap();
    if let Some(items) = value.as_array_mut() {
        for item in items {
            if let Some(location) = item.get_mut("location") {
                let text = location.as_str().unwrap_or("").to_string();
                *location =
                    serde_json::Value::String(text.split(':').next().unwrap_or(&text).to_string());
            }
        }
    }
    value
}

/// Run `fixture` through every default rule. `rule` must fire and no rule
/// outside `companions` may; returns only the findings of `rule`.
fn assert_fires(fixture: &str, rule: &str, companions: &[&str]) -> Vec<RuleViolation> {
    let registry = RuleRegistry::with_default_rules();
    assert!(
        registry.available_rules().contains(&rule),
        "`{rule}` is not registered in the default rule set"
    );

    let source = read_fixture(fixture);
    let all = registry.run_all(&source);
    let unexpected: Vec<String> = all
        .iter()
        .filter(|v| v.rule_name != rule && !companions.contains(&v.rule_name.as_str()))
        .map(|v| format!("{}: {}", v.rule_name, v.message))
        .collect();
    assert!(
        unexpected.is_empty(),
        "{fixture} should trigger `{rule}` (plus {companions:?}), but also triggered {unexpected:#?}"
    );

    let own: Vec<RuleViolation> = all.into_iter().filter(|v| v.rule_name == rule).collect();
    assert!(!own.is_empty(), "`{rule}` did not fire on {fixture}");
    assert_eq!(registry.run_by_name(&source, rule).len(), own.len());
    own
}

#[test]
fn reentrancy_fixture_fires_reentrancy() {
    // `invoke_contract` is also what `raw_invoke_contract` looks for.
    let violations = assert_fires(
        "rule_reentrancy_payout_router.rs",
        "reentrancy",
        &["raw_invoke_contract"],
    );

    assert_eq!(violations.len(), 1);
    assert!(violations[0].location.starts_with("settle"));
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("fixture_reentrancy", violations_json(&violations));
    });
}

#[test]
fn raw_invoke_contract_fixture_fires_only_raw_invoke_contract() {
    let violations = assert_fires(
        "rule_raw_invoke_contract_oracle_reader.rs",
        "raw_invoke_contract",
        &[],
    );

    // Only `read_price`; `read_price_checked` uses `try_invoke_contract`.
    assert_eq!(violations.len(), 1);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("fixture_raw_invoke_contract", violations_json(&violations));
    });
}

#[test]
fn panic_detection_fixture_fires_only_panic_detection() {
    let violations = assert_fires("rule_panic_detection_escrow.rs", "panic_detection", &[]);

    // panic!, unwrap and expect in the contract impl; the test module is skipped.
    let mut kinds: Vec<&str> = violations
        .iter()
        .map(|v| match v.location.as_str() {
            "release" => "panic!",
            "released" => "unwrap",
            "beneficiary_of" => "expect",
            other => panic!("unexpected location {other}"),
        })
        .collect();
    kinds.sort_unstable();
    assert_eq!(kinds, ["expect", "panic!", "unwrap"]);
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("fixture_panic_detection", violations_json(&violations));
    });
}
