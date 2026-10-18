//! Fixture-backed regression tests for individual detector rules.
//!
//! Each rule is exercised end to end: a contract-shaped fixture from
//! `tests/fixtures/` is run through the default [`RuleRegistry`], the same
//! dispatch path the CLI uses, rather than through the rule's own `check`.
//! The fixture must make exactly the rule under test fire, and the result is
//! snapshotted in `tests/snapshots/`. If a refactor of `rules/mod.rs` stops
//! invoking one of these rules, its test here fails.
//!
//! Regenerate snapshots after an intentional change:
//!   INSTA_UPDATE=always cargo test -p sanctifier-core --test rule_fixture_regressions_test

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

/// Run `fixture` through every default rule and check that `rule` is the only
/// one that fires, both on its own and through `run_by_name`.
fn assert_fires_alone(fixture: &str, rule: &str) -> Vec<RuleViolation> {
    let registry = RuleRegistry::with_default_rules();
    assert!(
        registry.available_rules().contains(&rule),
        "`{rule}` is not registered in the default rule set"
    );

    let source = read_fixture(fixture);
    let violations = registry.run_all(&source);
    assert!(!violations.is_empty(), "`{rule}` did not fire on {fixture}");
    let others: Vec<String> = violations
        .iter()
        .filter(|v| v.rule_name != rule)
        .map(|v| format!("{}: {}", v.rule_name, v.message))
        .collect();
    assert!(
        others.is_empty(),
        "{fixture} should trigger only `{rule}`, but also triggered {others:#?}"
    );

    let by_name = registry.run_by_name(&source, rule);
    assert_eq!(by_name.len(), violations.len());
    violations
}

#[test]
fn auth_gap_fixture_fires_only_auth_gap() {
    let violations = assert_fires_alone("rule_auth_gap_vault.rs", "auth_gap");

    assert_eq!(violations.len(), 1);
    assert!(violations[0].location.starts_with("reset_limit"));
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("fixture_auth_gap", violations_json(&violations));
    });
}

#[test]
fn arithmetic_overflow_fixture_fires_only_arithmetic_overflow() {
    let violations =
        assert_fires_alone("rule_arithmetic_overflow_ledger.rs", "arithmetic_overflow");

    assert_eq!(violations.len(), 1);
    assert!(violations[0].location.starts_with("credit"));
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("fixture_arithmetic_overflow", violations_json(&violations));
    });
}

#[test]
fn deprecated_sdk_usage_fixture_fires_only_deprecated_sdk_usage() {
    let violations = assert_fires_alone(
        "rule_deprecated_sdk_usage_registry.rs",
        "deprecated_sdk_usage",
    );

    assert_eq!(violations.len(), 1);
    assert!(violations[0].location.starts_with("register"));
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("fixture_deprecated_sdk_usage", violations_json(&violations));
    });
}

#[test]
fn a_rule_missing_from_the_registry_is_caught() {
    // Guards the guard: with the rule left out of the registry the fixture is
    // clean, so the tests above would fail rather than pass vacuously.
    let source = read_fixture("rule_auth_gap_vault.rs");
    let registry = RuleRegistry::new();
    assert!(registry.run_all(&source).is_empty());
    assert!(registry.run_by_name(&source, "auth_gap").is_empty());
}
