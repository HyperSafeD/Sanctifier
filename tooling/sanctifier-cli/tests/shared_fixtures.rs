#![allow(deprecated)]

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    // Canonicalize so the path has no `..` components: the analyzer treats a
    // path containing `..` as untrusted and falls back to scanning the current
    // directory, which would make a single-file fixture assertion scan the whole
    // tree. A real absolute path keeps each test scoped to its one fixture.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|e| panic!("fixture {name} should exist: {e}"))
}

#[test]
fn analyze_root_auth_gap_fixture_reports_one_s001() {
    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(fixture_path("auth_gap_contract.rs"))
        .arg("--format")
        .arg("json")
        .arg("--exit-code")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    let violations = json["rule_violations"].as_array().unwrap();
    let auth_gap_count = violations
        .iter()
        .filter(|v| v["rule_name"] == "auth_gap")
        .count();
    assert!(
        auth_gap_count >= 1,
        "Expected at least 1 auth_gap violation"
    );
}

#[test]
fn analyze_root_clean_token_fixture_reports_zero_findings() {
    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(fixture_path("clean_token.rs"))
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["summary"]["total_findings"], 0);
}

#[test]
fn analyze_root_overflow_fixture_reports_s003() {
    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(fixture_path("overflow_contract.rs"))
        .arg("--format")
        .arg("json")
        .arg("--exit-code")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    let violations = json["rule_violations"].as_array().unwrap();
    let overflow_count = violations
        .iter()
        .filter(|v| v["rule_name"] == "arithmetic_overflow")
        .count();
    assert!(
        overflow_count >= 1,
        "Expected at least 1 arithmetic_overflow violation"
    );
}
