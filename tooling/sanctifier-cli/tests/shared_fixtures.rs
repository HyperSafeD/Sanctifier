use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
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
    assert_eq!(json["auth_gaps"].as_array().unwrap().len(), 1);
    assert_eq!(json["summary"]["has_critical"], true);
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
    assert_eq!(json["arithmetic_issues"].as_array().unwrap().len(), 1);
}
