//! Integration tests for the core `Analyzer` against mock contracts.
//!
//! These tests verify the analyzer produces correct SARIF/JSON output
//! when run against minimal Soroban contracts with known vulnerabilities.

use sanctifier_core::{parser, Analyzer, RuleRegistry};
use std::path::PathBuf;

// ── Fixture helpers ────────────────────────────────────────────────────────────

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

// ── Analyzer integration tests ─────────────────────────────────────────────────

#[test]
fn analyzer_detects_auth_gap_in_fixture() {
    let fixture = fixture_path("auth_gap_contract.rs");
    let analyzer = Analyzer::new_with_defaults();
    let rule_registry = RuleRegistry::default();

    match analyzer.analyze_path(&fixture, &rule_registry) {
        Ok(findings) => {
            assert!(
                !findings.is_empty(),
                "analyzer should detect findings in auth_gap_contract"
            );
            let auth_findings: Vec<_> = findings
                .iter()
                .filter(|f| f.code == "S001")
                .collect();
            assert!(
                !auth_findings.is_empty(),
                "should detect S001 (auth gap) in auth_gap_contract"
            );
        }
        Err(e) => panic!("analyzer failed: {}", e),
    }
}

#[test]
fn analyzer_detects_overflow_in_fixture() {
    let fixture = fixture_path("overflow_contract.rs");
    let analyzer = Analyzer::new_with_defaults();
    let rule_registry = RuleRegistry::default();

    match analyzer.analyze_path(&fixture, &rule_registry) {
        Ok(findings) => {
            let overflow_findings: Vec<_> = findings
                .iter()
                .filter(|f| f.code == "S003")
                .collect();
            assert!(
                !overflow_findings.is_empty(),
                "should detect S003 (overflow) in overflow_contract"
            );
        }
        Err(e) => panic!("analyzer failed: {}", e),
    }
}

#[test]
fn analyzer_passes_clean_contract() {
    let fixture = fixture_path("clean_token.rs");
    let analyzer = Analyzer::new_with_defaults();
    let rule_registry = RuleRegistry::default();

    match analyzer.analyze_path(&fixture, &rule_registry) {
        Ok(findings) => {
            let critical_findings: Vec<_> = findings
                .iter()
                .filter(|f| f.severity == "Critical" || f.severity == "High")
                .collect();
            assert!(
                critical_findings.is_empty(),
                "clean_token should have no critical/high findings"
            );
        }
        Err(e) => panic!("analyzer failed: {}", e),
    }
}

#[test]
fn analyzer_json_output_is_valid() {
    let fixture = fixture_path("minimal_contract.rs");
    let analyzer = Analyzer::new_with_defaults();
    let rule_registry = RuleRegistry::default();

    match analyzer.analyze_path(&fixture, &rule_registry) {
        Ok(findings) => {
            let json = serde_json::to_string(&findings);
            assert!(json.is_ok(), "analyzer output should serialize to valid JSON");
        }
        Err(e) => panic!("analyzer failed: {}", e),
    }
}

#[test]
fn analyzer_handles_missing_file_gracefully() {
    let nonexistent = fixture_path("nonexistent_contract.rs");
    let analyzer = Analyzer::new_with_defaults();
    let rule_registry = RuleRegistry::default();

    let result = analyzer.analyze_path(&nonexistent, &rule_registry);
    assert!(result.is_err(), "analyzer should error on missing file");
}
