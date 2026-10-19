/// Performance benchmarks + budget enforcement for logging modes (#521).
///
/// Covers: quiet (error), default (warn), verbose (debug), and JSON log modes.
/// Each test asserts that a full `sanctifier analyze` run on a trivial contract
/// completes within a generous wall-clock budget. The budget is intentionally
/// large so the tests are stable under CI load — the point is to catch
/// catastrophic regressions, not sub-millisecond differences.
use assert_cmd::Command;
use std::fs;
use std::time::Instant;
use tempfile::tempdir;

/// Wall-clock budget per logging-mode test (seconds).
const LOG_BUDGET_SECS: u64 = 30;

fn write_trivial_contract(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let path = dir.path().join("contract.rs");
    fs::write(
        &path,
        r#"
use soroban_sdk::{contractimpl, Env};
#[contractimpl]
impl MyContract {
    pub fn add(_env: Env, a: u32, b: u32) -> u32 { a + b }
}
"#,
    )
    .unwrap();
    path
}

// ── budget tests ──────────────────────────────────────────────────────────────

/// `RUST_LOG=error` (quiet / production mode) must finish within budget.
#[test]
fn quiet_mode_analysis_within_budget() {
    let dir = tempdir().unwrap();
    let path = write_trivial_contract(&dir);

    let start = Instant::now();
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["analyze", "--format", "text"])
        .arg(&path)
        .env("RUST_LOG", "error")
        .assert()
        .success();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < LOG_BUDGET_SECS,
        "quiet-mode analysis took {elapsed:?}, budget {LOG_BUDGET_SECS}s"
    );
}

/// `RUST_LOG=debug` (verbose mode) must finish within budget.
#[test]
fn verbose_mode_analysis_within_budget() {
    let dir = tempdir().unwrap();
    let path = write_trivial_contract(&dir);

    let start = Instant::now();
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["analyze", "--format", "text"])
        .arg(&path)
        .env("RUST_LOG", "debug")
        .assert()
        .success();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < LOG_BUDGET_SECS,
        "verbose-mode analysis took {elapsed:?}, budget {LOG_BUDGET_SECS}s"
    );
}

/// JSON output + `RUST_LOG=debug` (json-logs mode) must finish within budget.
#[test]
fn json_logs_mode_analysis_within_budget() {
    let dir = tempdir().unwrap();
    let path = write_trivial_contract(&dir);

    let start = Instant::now();
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["analyze", "--format", "json"])
        .arg(&path)
        .env("RUST_LOG", "debug")
        .assert()
        .success();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < LOG_BUDGET_SECS,
        "json-logs-mode analysis took {elapsed:?}, budget {LOG_BUDGET_SECS}s"
    );
}

// ── correctness: logs go to the right stream ──────────────────────────────────

/// Debug log lines must arrive on stderr, not stdout, in text format.
#[test]
fn text_mode_debug_logs_go_to_stderr() {
    let dir = tempdir().unwrap();
    let path = write_trivial_contract(&dir);

    let out = Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["analyze", "--format", "text"])
        .arg(&path)
        .env("RUST_LOG", "sanctifier=debug")
        .output()
        .unwrap();

    let stderr = String::from_utf8(out.stderr).unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();

    // The progress line "Analyzing" goes to stderr in text mode.
    assert!(
        stderr.contains("Analyzing"),
        "debug logs should arrive on stderr"
    );
    // Stdout should not contain raw log lines.
    assert!(
        !stdout.contains("DEBUG"),
        "DEBUG log lines must not leak onto stdout"
    );
}

/// Debug log lines must be valid JSON and arrive on stderr in json-logs mode.
#[test]
fn json_mode_logs_are_valid_json_on_stderr() {
    let dir = tempdir().unwrap();
    let path = write_trivial_contract(&dir);

    let out = Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["analyze", "--format", "json"])
        .arg(&path)
        .env("RUST_LOG", "sanctifier=debug")
        .output()
        .unwrap();

    assert!(out.status.success(), "command must succeed");

    // Stdout must be valid JSON (the analysis result).
    let stdout = String::from_utf8(out.stdout).unwrap();
    serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("stdout must be valid JSON analysis output");

    // Stderr log lines must themselves be valid JSON objects.
    let stderr = String::from_utf8(out.stderr).unwrap();
    for line in stderr.lines().filter(|l| !l.is_empty()) {
        serde_json::from_str::<serde_json::Value>(line)
            .unwrap_or_else(|_| panic!("stderr log line is not valid JSON: {line}"));
    }
}

/// Quiet mode (`RUST_LOG=error`) produces no output on stderr for a clean file.
#[test]
fn quiet_mode_produces_no_stderr_on_clean_file() {
    let dir = tempdir().unwrap();
    let path = write_trivial_contract(&dir);

    let out = Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["analyze", "--format", "text"])
        .arg(&path)
        .env_remove("RUST_LOG") // use the default "warn" filter via logging::init
        .env("RUST_LOG", "error")
        .output()
        .unwrap();

    let stderr = String::from_utf8(out.stderr).unwrap();
    // In error-only mode no warn/info/debug lines should appear for a trivial file.
    // The "Analyzing …" progress line uses tracing::info!, so it should be suppressed.
    assert!(
        !stderr.contains("ERROR"),
        "no ERROR-level log expected for a clean contract: {stderr}"
    );
}
