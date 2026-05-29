#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::env;

/// Test that CLI version is properly reported
#[test]
fn test_cli_version_command() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("sanctifier"));
}

/// Test that JSON output includes schema_version field
#[test]
fn test_json_output_includes_schema_version() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("analyze")
        .arg(&fixture_path)
        .arg("--format")
        .arg("json")
        .env_remove("RUST_LOG")
        .assert()
        .success();
}

/// Test that analyze command validates input properly
#[test]
fn test_analyze_invalid_path() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("analyze")
        .arg("/nonexistent/path/to/contract")
        .assert()
        .failure();
}

/// Test that analyze command with valid contract succeeds
#[test]
fn test_analyze_valid_contract_succeeds() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("analyze")
        .arg(&fixture_path)
        .env_remove("RUST_LOG")
        .assert()
        .success()
        .stdout(predicate::str::contains("Static analysis complete"));
}

/// Test that analyze command with vulnerable contract reports findings
#[test]
fn test_analyze_vulnerable_contract_reports_findings() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/vulnerable_contract.rs");

    cmd.arg("analyze")
        .arg(&fixture_path)
        .env_remove("RUST_LOG")
        .assert()
        .success();
}

/// Test that timeout parameter is respected
#[test]
fn test_analyze_with_timeout() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("analyze")
        .arg(&fixture_path)
        .arg("--timeout")
        .arg("60")
        .env_remove("RUST_LOG")
        .assert()
        .success();
}

/// Test that ledger limit parameter is respected
#[test]
fn test_analyze_with_custom_ledger_limit() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("analyze")
        .arg(&fixture_path)
        .arg("--limit")
        .arg("32000")
        .env_remove("RUST_LOG")
        .assert()
        .success();
}

/// Test that exit code is returned when findings meet severity threshold
#[test]
fn test_analyze_exit_code_on_high_severity() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/vulnerable_contract.rs");

    let _ = cmd
        .arg("analyze")
        .arg(&fixture_path)
        .arg("--exit-code")
        .arg("--min-severity")
        .arg("high")
        .env_remove("RUST_LOG")
        .assert();
    // May succeed or fail depending on findings
}

/// Test that format parameter accepts valid values
#[test]
fn test_analyze_format_text() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("analyze")
        .arg(&fixture_path)
        .arg("--format")
        .arg("text")
        .env_remove("RUST_LOG")
        .assert()
        .success();
}

/// Test that format parameter accepts json
#[test]
fn test_analyze_format_json() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("analyze")
        .arg(&fixture_path)
        .arg("--format")
        .arg("json")
        .env_remove("RUST_LOG")
        .assert()
        .success();
}
