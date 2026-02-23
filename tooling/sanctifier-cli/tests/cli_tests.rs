use assert_cmd::Command;
use predicates::prelude::*;
use std::env;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: sanctifier"));
}

#[test]
fn test_analyze_valid_contract() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("analyze")
        .arg(fixture_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Static analysis complete."))
        .stdout(predicate::str::contains("No ledger size issues found."))
        .stdout(predicate::str::contains("No upgrade pattern issues found."));
}

#[test]
fn test_analyze_vulnerable_contract() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/vulnerable_contract.rs");

    cmd.arg("analyze")
        .arg(fixture_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Found potential Authentication Gaps!"))
        .stdout(predicate::str::contains("Found explicit Panics/Unwraps!"))
        .stdout(predicate::str::contains("Found unchecked Arithmetic Operations!"));
}

#[test]
fn test_analyze_json_output() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    let assert = cmd.arg("analyze")
        .arg(fixture_path)
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    
    // JSON starts with {
    assert.stdout(predicate::str::starts_with("{"));
}

#[test]
fn test_analyze_empty_macro_heavy() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/macro_heavy.rs");

    cmd.arg("analyze")
        .arg(fixture_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Static analysis complete."));
}
