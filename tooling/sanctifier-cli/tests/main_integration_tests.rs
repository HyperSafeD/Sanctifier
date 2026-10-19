/// Integration tests for `sanctifier-cli/src/main.rs` (#1356).
///
/// This file drives the compiled `sanctifier` binary against a mock workspace
/// and asserts on SARIF/JSON output, CLI flag wiring, and edge-case behaviour
/// that is exercised through `main()` (logging, colour, network banner, exit
/// codes) rather than being covered by the per-command unit tests.
///
/// Conventions:
/// - One logical scenario per `#[test]` function.
/// - Heavy test fixtures are written to `tempdir()` so tests are hermetic.
/// - JSON/SARIF assertions decode the output with `serde_json` rather than
///   relying on fragile string matching.
#[allow(deprecated)]
use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Write a minimal Soroban contract with an auth-gap (missing `require_auth`)
/// to `<dir>/lib.rs` and return the path.
fn write_auth_gap_contract(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("lib.rs");
    fs::write(
        &path,
        r#"#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Address, String};

#[contract]
pub struct VulnerableContract;

#[contractimpl]
impl VulnerableContract {
    pub fn set_admin(env: Env, admin: Address) {
        env.storage()
            .instance()
            .set(&String::from_slice(&env, "admin"), &admin);
        // Missing require_auth() — intentional auth-gap fixture.
    }
}
"#,
    )
    .unwrap();
    path
}

/// Write a clean contract with no findings to `<dir>/clean.rs`.
fn write_clean_contract(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("clean.rs");
    fs::write(&path, "// empty\npub fn noop() {}\n").unwrap();
    path
}

/// Decode stdout bytes to a `serde_json::Value`, panicking on failure.
fn parse_json_stdout(bytes: &[u8]) -> Value {
    let text = String::from_utf8(bytes.to_vec()).expect("stdout must be UTF-8");
    serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!("stdout is not valid JSON ({e}):\n---\n{text}\n---")
    })
}

// ── main() bootstrapping ─────────────────────────────────────────────────────

/// Verify that `main()` handles an unrecognised subcommand without panicking.
#[test]
fn main_unknown_subcommand_exits_nonzero_without_panic() {
    Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("not-a-real-command-xyz")
        .assert()
        .failure();
}

/// `--no-color` is forwarded to the colour layer before any subcommand runs.
#[test]
fn main_no_color_flag_is_accepted() {
    let dir = tempdir().unwrap();
    let contract = write_clean_contract(dir.path());

    Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("--no-color")
        .arg("analyze")
        .arg(&contract)
        .env_remove("RUST_LOG")
        .assert()
        .success();
}

/// `--network mainnet` produces the MAINNET banner on stderr (not stdout).
#[test]
fn main_network_flag_mainnet_emits_banner_on_stderr() {
    let dir = tempdir().unwrap();
    let contract = write_clean_contract(dir.path());

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["--network", "mainnet"])
        .arg("analyze")
        .arg(&contract)
        .env_remove("RUST_LOG")
        .assert()
        .success()
        .stderr(predicates::str::contains("MAINNET"));
}

/// `--network futurenet` produces the FUTURENET banner on stderr.
#[test]
fn main_network_flag_futurenet_emits_banner_on_stderr() {
    let dir = tempdir().unwrap();
    let contract = write_clean_contract(dir.path());

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["--network", "futurenet"])
        .arg("analyze")
        .arg(&contract)
        .env_remove("RUST_LOG")
        .assert()
        .success()
        .stderr(predicates::str::contains("FUTURENET"));
}

/// When `--format json` is used the network banner is suppressed from stderr
/// so that stderr stays machine-parseable.
#[test]
fn main_json_format_suppresses_network_banner_from_stderr() {
    let dir = tempdir().unwrap();
    let contract = write_clean_contract(dir.path());

    // The banner would normally appear; with --format json it must not.
    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["analyze", "--format", "json"])
        .arg(&contract)
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        !stderr.contains("TESTNET") && !stderr.contains("Sanctifier —"),
        "network banner must be absent from stderr in JSON mode, got:\n{stderr}"
    );
}

// ── Mock workspace → JSON output ─────────────────────────────────────────────

/// A mock workspace with an auth-gap contract produces valid JSON output whose
/// `rule_violations` array contains at least one `auth_gap` entry.
#[test]
fn main_mock_workspace_json_contains_auth_gap_violation() {
    let dir = tempdir().unwrap();
    let contract = write_auth_gap_contract(dir.path());

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract)
        .args(["--format", "json"])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    assert!(output.status.success(), "analyze should exit 0");

    let json = parse_json_stdout(&output.stdout);
    let violations = json["rule_violations"]
        .as_array()
        .expect("rule_violations must be an array");

    let auth_gap = violations
        .iter()
        .find(|v| v["rule_name"] == "auth_gap")
        .expect("expected an auth_gap violation in rule_violations");

    assert!(
        auth_gap["location"].is_string(),
        "auth_gap violation must carry a location"
    );
    assert!(
        auth_gap["location"]
            .as_str()
            .unwrap()
            .contains("set_admin"),
        "auth_gap location should reference the vulnerable function"
    );
}

/// The `error_codes` catalog included in JSON output must contain S001 for an
/// auth-gap contract.
#[test]
fn main_mock_workspace_json_error_codes_catalog_includes_s001() {
    let dir = tempdir().unwrap();
    let contract = write_auth_gap_contract(dir.path());

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract)
        .args(["--format", "json"])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    let json = parse_json_stdout(&output.stdout);
    let codes = json["error_codes"]
        .as_array()
        .expect("error_codes must be an array");

    assert!(
        codes.iter().any(|c| c["code"] == "S001"),
        "error_codes catalog must include S001 (AUTH_GAP)"
    );
}

/// A clean contract produces JSON with an empty `rule_violations` array.
#[test]
fn main_clean_contract_json_has_empty_violations() {
    let dir = tempdir().unwrap();
    let contract = write_clean_contract(dir.path());

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract)
        .args(["--format", "json"])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    assert!(output.status.success());

    let json = parse_json_stdout(&output.stdout);
    let violations = json["rule_violations"]
        .as_array()
        .expect("rule_violations must be present and be an array");

    assert!(
        violations.is_empty(),
        "clean contract must produce zero violations"
    );
}

// ── Mock workspace → SARIF output ────────────────────────────────────────────

/// SARIF output for an auth-gap contract is a well-formed 2.1.0 document
/// whose `runs[0].results` contains a result with `ruleId == "auth_gap"`.
#[test]
fn main_mock_workspace_sarif_contains_auth_gap_result() {
    let dir = tempdir().unwrap();
    let contract = write_auth_gap_contract(dir.path());

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract)
        .args(["--format", "sarif"])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    assert!(output.status.success(), "sarif output should exit 0");

    let json = parse_json_stdout(&output.stdout);
    assert_eq!(
        json["version"], "2.1.0",
        "SARIF version must be 2.1.0"
    );

    let results = json["runs"][0]["results"]
        .as_array()
        .expect("sarif runs[0].results must be an array");

    let auth_gap = results
        .iter()
        .find(|r| r["ruleId"] == "auth_gap")
        .expect("expected a SARIF result with ruleId 'auth_gap'");

    assert!(
        auth_gap["level"].is_string(),
        "auth_gap sarif result must have a level"
    );
    assert!(
        auth_gap["message"]["text"].is_string(),
        "auth_gap sarif result must have message.text"
    );
    assert!(
        auth_gap["locations"].is_array(),
        "auth_gap sarif result must have locations"
    );
}

/// SARIF output for a clean contract still exits 0 and has an empty results
/// array.
#[test]
fn main_clean_contract_sarif_has_empty_results() {
    let dir = tempdir().unwrap();
    let contract = write_clean_contract(dir.path());

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract)
        .args(["--format", "sarif"])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    assert!(output.status.success());

    let json = parse_json_stdout(&output.stdout);
    let results = json["runs"][0]["results"]
        .as_array()
        .expect("sarif runs[0].results must be an array");

    assert!(
        results.is_empty(),
        "clean contract must produce zero SARIF results"
    );
}

/// SARIF `runs[0].tool.driver.name` must always be `"sanctifier"`.
#[test]
fn main_sarif_tool_driver_name_is_sanctifier() {
    let dir = tempdir().unwrap();
    let contract = write_clean_contract(dir.path());

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract)
        .args(["--format", "sarif"])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    let json = parse_json_stdout(&output.stdout);
    assert_eq!(
        json["runs"][0]["tool"]["driver"]["name"],
        "sanctifier",
        "tool.driver.name must be 'sanctifier'"
    );
}

/// SARIF physical-location URIs for findings must reference the analysed file.
#[test]
fn main_sarif_physical_location_uri_references_analysed_file() {
    let dir = tempdir().unwrap();
    let contract = write_auth_gap_contract(dir.path());

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract)
        .args(["--format", "sarif"])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    let json = parse_json_stdout(&output.stdout);
    let results = json["runs"][0]["results"].as_array().unwrap();
    let first = &results[0];
    let uri = first["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
        .as_str()
        .expect("physicalLocation.artifactLocation.uri must be a string");

    // The URI should contain the file name we wrote.
    assert!(
        uri.contains("lib.rs"),
        "location URI must reference the analysed file, got: {uri}"
    );
}

// ── Edge cases ────────────────────────────────────────────────────────────────

/// Providing a non-existent path exits with code 2 (ERROR) regardless of the
/// output format.
#[test]
fn main_nonexistent_path_exits_code_2_for_json_format() {
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["analyze", "--format", "json"])
        .arg("does/not/exist.rs")
        .assert()
        .code(2);
}

/// `--format sarif` for a non-existent path also exits with code 2.
#[test]
fn main_nonexistent_path_exits_code_2_for_sarif_format() {
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["analyze", "--format", "sarif"])
        .arg("no-such-path.rs")
        .assert()
        .code(2);
}

/// `--format ndjson` emits at least one line and that last line is the done
/// event with a `total_findings` key — exercising the streaming path wired up
/// in `main()`.
#[test]
fn main_ndjson_format_emits_done_event_as_last_line() {
    let dir = tempdir().unwrap();
    let contract = write_clean_contract(dir.path());

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["analyze", "--format", "ndjson"])
        .arg(&contract)
        .output()
        .unwrap();

    assert!(output.status.success(), "ndjson output should exit 0");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty(), "ndjson output must have at least one line");

    let last: Value = serde_json::from_str(lines.last().unwrap())
        .expect("last NDJSON line must be valid JSON");

    assert_eq!(
        last["event"], "done",
        "last ndjson line must be the done event"
    );
    assert!(
        last["total_findings"].is_number(),
        "done event must carry total_findings"
    );
}

/// With `--profile ci` and findings present the process exits 1 (FINDINGS_FOUND).
#[test]
fn main_profile_ci_exits_1_when_findings_present() {
    let dir = tempdir().unwrap();
    let contract = write_auth_gap_contract(dir.path());

    Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract)
        .args(["--profile", "ci"])
        .env_remove("RUST_LOG")
        .assert()
        .code(1);
}

/// With `--profile ci` and a clean contract the process exits 0 (SUCCESS).
#[test]
fn main_profile_ci_exits_0_when_no_findings() {
    let dir = tempdir().unwrap();
    let contract = write_clean_contract(dir.path());

    Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract)
        .args(["--profile", "ci"])
        .env_remove("RUST_LOG")
        .assert()
        .code(0);
}

/// An unknown format string is rejected with an "unknown output format" error
/// message on stderr, exercising the format validation path in `run()`.
#[test]
fn main_unknown_format_rejected_with_error_message() {
    let dir = tempdir().unwrap();
    let contract = write_clean_contract(dir.path());

    Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract)
        .args(["--format", "xml"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown output format"));
}

/// `sanctifier completions bash` is wired through `main()` and must produce
/// a bash completion script on stdout.
#[test]
fn main_completions_subcommand_writes_to_stdout() {
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicates::str::contains("sanctifier"));
}

/// `sanctifier version` is wired through `main()` and produces version info.
#[test]
fn main_version_subcommand_prints_version() {
    Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("version")
        .assert()
        .success()
        .stdout(predicates::str::contains("Sanctifier"));
}
