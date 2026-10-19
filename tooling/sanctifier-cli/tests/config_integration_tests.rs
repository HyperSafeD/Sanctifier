#![allow(deprecated)]

use assert_cmd::Command;
use sanctifier_cli::config::Config;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn write_mock_workspace(root: &std::path::Path) -> std::path::PathBuf {
    let src = root.join("contract.rs");
    fs::write(
        &src,
        r#"#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String};

#[contract]
pub struct MockWorkspaceContract;

#[contractimpl]
impl MockWorkspaceContract {
    pub fn set_admin(env: Env, admin: Address) {
        env.storage()
            .instance()
            .set(&String::from_str(&env, "admin"), &admin);
    }
}
"#,
    )
    .unwrap();
    src
}
fn run_with_config(format: &str) -> Value {
    let dir = tempdir().unwrap();
    let contract = write_mock_workspace(dir.path());
    let config_path = dir.path().join("sanctifier.toml");
    fs::write(
        &config_path,
        format!(
            "paths = [\"{}\"]\noutput_format = \"{}\"\nseverity_threshold = \"low\"\n",
            contract.display(),
            format
        ),
    )
    .unwrap();

    let config = Config::from_file(&config_path).expect("config should load");
    assert_eq!(config.paths(), &[contract.clone()]);
    assert_eq!(config.output_format(), format);

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&config.paths()[0])
        .args(["--format", config.output_format()])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(1),
        "the vulnerable fixture should produce findings, not a CLI/runtime error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("machine-readable output must be valid JSON")
}
#[test]
fn config_drives_json_output_for_mock_workspace() {
    let json = run_with_config("json");
    assert!(json["rule_violations"].is_array());
    assert!(json["rule_violations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["rule_name"] == "auth_gap"));
}

#[test]
fn config_drives_sarif_output_for_mock_workspace() {
    let sarif = run_with_config("sarif");
    assert_eq!(sarif["version"], "2.1.0");
    let results = sarif["runs"][0]["results"]
        .as_array()
        .expect("SARIF results must be an array");
    assert!(
        results.iter().any(|result| {
            result["ruleId"] == "auth_gap"
                || result["ruleId"]
                    .as_str()
                    .is_some_and(|id| id.eq_ignore_ascii_case("S001"))
        }),
        "expected auth-gap finding in SARIF output"
    );
}
