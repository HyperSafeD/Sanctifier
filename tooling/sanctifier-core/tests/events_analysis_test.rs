//! Unit tests for the S008 event analysis pass (gas/correctness).
//!
//! Covers:
//! * Inconsistent topic-count detection across emit sites for the same event.
//! * Gas-optimization hints when string literals are used instead of `symbol_short!`.
//! * Zero-finding guarantee on well-formed event code.
//! * Empty / non-event source produces no findings.
//! * Multiple distinct events with matching schemas are not flagged.

use sanctifier_core::{Analyzer, SanctifyConfig};
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {name} not readable: {e}"))
}

// ── Fixture-based tests ──────────────────────────────────────────────────────

#[test]
fn inconsistent_event_fixture_emits_schema_issue() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = fixture("events_inconsistent_contract.rs");
    let issues = analyzer.scan_events(&source);

    let schema_issues: Vec<_> = issues
        .iter()
        .filter(|i| i.issue_type == "inconsistent_schema")
        .collect();
    assert!(
        !schema_issues.is_empty(),
        "expected at least one InconsistentSchema finding; got: {issues:?}"
    );
}

#[test]
fn string_topic_fixture_emits_gas_optimization_hint() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = fixture("events_inconsistent_contract.rs");
    let issues = analyzer.scan_events(&source);

    let gas_issues: Vec<_> = issues
        .iter()
        .filter(|i| i.issue_type == "optimizable_topic")
        .collect();
    assert!(
        !gas_issues.is_empty(),
        "expected at least one OptimizableTopic finding; got: {issues:?}"
    );
}

#[test]
fn clean_event_fixture_produces_no_findings() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = fixture("events_clean_contract.rs");
    let issues = analyzer.scan_events(&source);
    assert!(
        issues.is_empty(),
        "clean event fixture should produce zero findings; got: {issues:?}"
    );
}

// ── Inline unit tests ────────────────────────────────────────────────────────

#[test]
fn empty_source_produces_no_event_issues() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let issues = analyzer.scan_events("");
    assert!(issues.is_empty());
}

#[test]
fn source_without_events_produces_no_issues() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        pub fn add(a: i128, b: i128) -> i128 { a + b }
    "#;
    let issues = analyzer.scan_events(source);
    assert!(issues.is_empty());
}

#[test]
fn consistent_multi_event_schemas_produce_no_findings() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    // Two events with different names — each internally consistent.
    let source = r#"
        pub fn mint(env: Env) {
            env.events().publish((symbol_short!("mint"), symbol_short!("to")), 1i128);
        }
        pub fn mint_again(env: Env) {
            env.events().publish((symbol_short!("mint"), symbol_short!("to")), 2i128);
        }
        pub fn burn(env: Env) {
            env.events().publish((symbol_short!("burn"), symbol_short!("from")), 3i128);
        }
    "#;
    let issues = analyzer.scan_events(source);
    assert!(
        issues.is_empty(),
        "consistent same-name events should not be flagged; got: {issues:?}"
    );
}

#[test]
fn inconsistent_topic_count_detected_inline() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    // "pay" emitted with 1 topic then 2 topics.
    let source = r#"
        pub fn pay_v1(env: Env) {
            env.events().publish(("pay", symbol_short!("src")), 10i128);
        }
        pub fn pay_v2(env: Env) {
            env.events().publish(("pay", symbol_short!("src"), symbol_short!("dst")), 20i128);
        }
    "#;
    let issues = analyzer.scan_events(source);
    let schema_issues: Vec<_> = issues
        .iter()
        .filter(|i| i.issue_type == "inconsistent_schema")
        .collect();
    assert!(
        !schema_issues.is_empty(),
        "inconsistent topic count should be detected"
    );
}

#[test]
fn gas_optimization_issue_carries_event_name() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        pub fn emit_bad(env: Env) {
            env.events().publish(("transfer", "amount"), 42i128);
        }
    "#;
    let issues = analyzer.scan_events(source);
    let gas_issues: Vec<_> = issues
        .iter()
        .filter(|i| i.issue_type == "optimizable_topic")
        .collect();
    assert!(
        !gas_issues.is_empty(),
        "string topic should trigger OptimizableTopic"
    );
    // The issue must reference some event/location
    assert!(
        gas_issues.iter().any(|i| !i.location.is_empty()),
        "gas issue should carry a location"
    );
}

#[test]
fn findings_carry_non_empty_message() {
    let analyzer = Analyzer::new(SanctifyConfig::default());
    let source = r#"
        pub fn bad(env: Env) {
            env.events().publish(("ev", "raw_string"), 1i128);
        }
    "#;
    let issues = analyzer.scan_events(source);
    for issue in &issues {
        assert!(
            !issue.message.is_empty(),
            "every finding must have a message"
        );
    }
}
