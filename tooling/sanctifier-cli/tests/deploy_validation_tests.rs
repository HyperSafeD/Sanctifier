/// Input validation tests for `sanctifier deploy` (#527).
///
/// Tests that runtime guard deploy UX returns structured E010/E011 errors
/// before attempting any I/O (build, WASM lookup, soroban-cli invocation).
use assert_cmd::Command;
use predicates::prelude::*;

// ── E010 — network validation ─────────────────────────────────────────────────

#[test]
#[ignore = "`deploy` subcommand (#527) is not yet implemented; tests target a non-existent command"]
fn deploy_rejects_unknown_network_with_e010() {
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["deploy", "--network", "devnet"])
        .env_remove("SOROBAN_SECRET_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E010"));
}

#[test]
#[ignore = "`deploy` subcommand (#527) is not yet implemented; tests target a non-existent command"]
fn deploy_rejects_empty_network_string_with_e010() {
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["deploy", "--network", ""])
        .env_remove("SOROBAN_SECRET_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E010"));
}

#[test]
#[ignore = "`deploy` subcommand (#527) is not yet implemented; tests target a non-existent command"]
fn deploy_rejects_staging_network_with_e010() {
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["deploy", "--network", "staging"])
        .env_remove("SOROBAN_SECRET_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E010"));
}

#[test]
#[ignore = "`deploy` subcommand (#527) is not yet implemented; hint-text assertions pending"]
fn deploy_e010_hint_lists_valid_networks() {
    let out = Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["deploy", "--network", "badnet"])
        .env_remove("SOROBAN_SECRET_KEY")
        .output()
        .unwrap();

    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("testnet"), "hint must mention testnet");
    assert!(stderr.contains("futurenet"), "hint must mention futurenet");
    assert!(stderr.contains("mainnet"), "hint must mention mainnet");
}

// ── E011 — credentials validation ─────────────────────────────────────────────

#[test]
#[ignore = "`deploy` subcommand (#527) is not yet implemented; tests target a non-existent command"]
fn deploy_rejects_missing_credentials_with_e011() {
    // Use a valid network so we get past E010 and reach the credentials check.
    // Current dir exists, so the path check (E001) passes too.
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["deploy", "--network", "testnet"])
        .env_remove("SOROBAN_SECRET_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("E011"));
}

#[test]
#[ignore = "`deploy` subcommand (#527) is not yet implemented; hint-text assertions pending"]
fn deploy_e011_hint_mentions_env_var() {
    let out = Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["deploy", "--network", "testnet"])
        .env_remove("SOROBAN_SECRET_KEY")
        .output()
        .unwrap();

    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("SOROBAN_SECRET_KEY"),
        "hint must reference the env var; got: {stderr}"
    );
    assert!(
        stderr.contains("--secret-key"),
        "hint must reference the flag; got: {stderr}"
    );
}

// ── valid networks pass validation ────────────────────────────────────────────

/// testnet/futurenet/mainnet should all pass network validation and proceed to
/// the next check (credentials). We verify they do NOT produce E010.
#[test]
#[ignore = "`deploy` subcommand (#527) is not yet implemented; tests target a non-existent command"]
fn deploy_valid_networks_do_not_produce_e010() {
    for network in &["testnet", "futurenet", "mainnet"] {
        let out = Command::cargo_bin("sanctifier")
            .unwrap()
            .args(["deploy", "--network", network])
            .env_remove("SOROBAN_SECRET_KEY")
            .output()
            .unwrap();

        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            !stderr.contains("E010"),
            "network '{network}' is valid and must not produce E010; stderr: {stderr}"
        );
    }
}
