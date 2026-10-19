//! Fixture-backed regression tests for the storage-related detector rules:
//! `missing_ttl_bump`, `missing_state_event`, `ledger_size` and
//! `instance_storage_misuse`.
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
//!   INSTA_UPDATE=always cargo test -p sanctifier-core --test rule_fixture_regressions_storage_test

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
fn missing_ttl_bump_fixture_fires_only_missing_ttl_bump() {
    let violations = assert_fires("rule_missing_ttl_bump_notebook.rs", "missing_ttl_bump", &[]);

    assert_eq!(violations.len(), 1);
    assert!(violations[0].location.starts_with("save_note"));
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("fixture_missing_ttl_bump", violations_json(&violations));
    });
}

#[test]
fn missing_state_event_fixture_fires_missing_state_event() {
    let violations = assert_fires(
        "rule_missing_state_event_pausable.rs",
        "missing_state_event",
        &["shadow_storage"],
    );

    assert_eq!(violations.len(), 1);
    assert!(violations[0].message.contains("'pause'"));
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("fixture_missing_state_event", violations_json(&violations));
    });
}

#[test]
fn ledger_size_fixture_fires_only_ledger_size() {
    let violations = assert_fires("rule_ledger_size_archive.rs", "ledger_size", &[]);

    assert_eq!(violations.len(), 1);
    assert!(violations[0].message.contains("PriceArchive"));
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!("fixture_ledger_size", violations_json(&violations));
    });
}

#[test]
fn instance_storage_misuse_fixture_fires_only_instance_storage_misuse() {
    let violations = assert_fires(
        "rule_instance_storage_misuse_ledger.rs",
        "instance_storage_misuse",
        &[],
    );

    assert_eq!(violations.len(), 1);
    assert!(violations[0].location.starts_with("deposit"));
    with_settings!({ sort_maps => true }, {
        insta::assert_json_snapshot!(
            "fixture_instance_storage_misuse",
            violations_json(&violations)
        );
    });
}

#[test]
fn an_empty_registry_finds_nothing_in_these_fixtures() {
    // Guards the guard: with no rules registered every fixture is clean, so the
    // tests above would fail rather than pass vacuously.
    let registry = RuleRegistry::new();
    for fixture in [
        "rule_missing_ttl_bump_notebook.rs",
        "rule_missing_state_event_pausable.rs",
        "rule_ledger_size_archive.rs",
        "rule_instance_storage_misuse_ledger.rs",
    ] {
        assert!(registry.run_all(&read_fixture(fixture)).is_empty());
    }
}
