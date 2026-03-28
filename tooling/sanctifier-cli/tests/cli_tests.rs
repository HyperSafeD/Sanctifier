#![allow(deprecated)]
use assert_cmd::Command;
use jsonschema::JSONSchema;
use mockito::Server;
use std::env;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage: sanctifier"));
}

#[test]
fn test_analyze_valid_contract() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("analyze")
        .arg(fixture_path)
        .env_remove("RUST_LOG")
        .assert()
        .success()
        // Progress indicator is written to stderr
        .stderr(predicates::str::contains("Analyzing"))
        .stdout(predicates::str::contains("Static analysis complete."))
        .stdout(predicates::str::contains("No ledger size issues found."))
        .stdout(predicates::str::contains(
            "No storage key collisions found.",
        ));
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
        .stdout(predicates::str::contains(
            "Found potential Authentication Gaps!",
        ))
        .stdout(predicates::str::contains("Found explicit Panics/Unwraps!"))
        .stdout(predicates::str::contains(
            "Found unchecked Arithmetic Operations!",
        ));
}

#[test]
fn test_analyze_json_output() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    let assert = cmd
        .arg("analyze")
        .arg(fixture_path)
        .arg("--format")
        .arg("json")
        .env_remove("RUST_LOG")
        .assert()
        .success();

    // JSON starts with {
    assert.stdout(predicates::str::starts_with("{"));
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
        .stdout(predicates::str::contains("Static analysis complete."));
}

#[test]
fn test_analyze_debug_logging_goes_to_stderr() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("analyze")
        .arg(fixture_path)
        .env("RUST_LOG", "sanctifier=debug")
        .assert()
        .success()
        .stderr(predicates::str::contains("Scanning Rust source file"))
        .stdout(predicates::str::contains("Static analysis complete."));
}

#[test]
fn test_analyze_json_logs_do_not_pollute_stdout() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("analyze")
        .arg(fixture_path)
        .arg("--format")
        .arg("json")
        .env("RUST_LOG", "sanctifier=debug")
        .assert()
        .success()
        .stdout(predicates::str::starts_with("{"))
        .stderr(predicates::str::contains("\"level\":\"DEBUG\""));
}

#[test]
fn test_update_help() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("update")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("latest Sanctifier binary"));
}

#[test]
fn test_init_creates_sanctify_toml_in_current_directory() {
    let temp_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();

    cmd.current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .success();

    let config_path = temp_dir.path().join(".sanctify.toml");
    assert!(
        config_path.exists(),
        "Expected init command to create .sanctify.toml"
    );
}

#[test]
fn test_init_fails_when_config_exists_without_force() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join(".sanctify.toml");
    fs::write(&config_path, "existing content").unwrap();

    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .failure();

    let content = fs::read_to_string(&config_path).unwrap();
    assert_eq!(content, "existing content");
}

#[test]
fn test_init_overwrites_when_force_is_set() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join(".sanctify.toml");
    fs::write(&config_path, "existing content").unwrap();

    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("init")
        .arg("--force")
        .assert()
        .success();

    let content = fs::read_to_string(&config_path).unwrap();
    assert_ne!(content, "existing content");
    assert!(content.contains("ignore_paths"));
}

/// Verifies that `sanctifier report <file>` prints a Markdown document to
/// stdout that contains all required top-level sections.
#[test]
fn test_report_markdown_stdout() {
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/vulnerable_contract.rs");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("report")
        .arg(fixture_path)
        .env_remove("RUST_LOG")
        .assert()
        .success()
        .stdout(predicates::str::contains("# Sanctifier Security Report"))
        .stdout(predicates::str::contains("## Summary"))
        .stdout(predicates::str::contains("## Findings"))
        .stdout(predicates::str::contains("**Contract path**"))
        .stdout(predicates::str::contains("**Analysis date**"))
        .stdout(predicates::str::contains("**Tool version**"));
}

/// Verifies that `sanctifier report --output <file>.md` writes a Markdown
/// document to disk with the expected content.
#[test]
fn test_report_writes_markdown_file() {
    let temp_dir = tempdir().unwrap();
    let out_path = temp_dir.path().join("report.md");
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/vulnerable_contract.rs");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("report")
        .arg(fixture_path)
        .arg("--output")
        .arg(&out_path)
        .env_remove("RUST_LOG")
        .assert()
        .success()
        .stdout(predicates::str::contains("Report written to"));

    let content = fs::read_to_string(&out_path).expect("report.md should have been created");
    assert!(
        content.contains("# Sanctifier Security Report"),
        "Markdown report should have an H1 header"
    );
    assert!(
        content.contains("## Summary"),
        "Markdown report should have a Summary section"
    );
    assert!(
        content.contains("## Findings"),
        "Markdown report should have a Findings section"
    );
}

/// Verifies that `sanctifier report --output <file>.html` writes an HTML
/// document with the expected structure.
#[test]
fn test_report_writes_html_file() {
    let temp_dir = tempdir().unwrap();
    let out_path = temp_dir.path().join("report.html");
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/vulnerable_contract.rs");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("report")
        .arg(fixture_path)
        .arg("--output")
        .arg(&out_path)
        .env_remove("RUST_LOG")
        .assert()
        .success();

    let content = fs::read_to_string(&out_path).expect("report.html should have been created");
    assert!(
        content.contains("<!DOCTYPE html>"),
        "HTML report should start with DOCTYPE"
    );
    assert!(
        content.contains("Sanctifier Security Report"),
        "HTML report should contain the title"
    );
    assert!(
        content.contains("<h2>Summary</h2>"),
        "HTML report should have a Summary heading"
    );
}

#[test]
fn test_webhook_failure_is_non_fatal() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/notify")
        .match_query(mockito::Matcher::UrlEncoded(
            "sanctifier_provider".into(),
            "discord".into(),
        ))
        .with_status(500)
        .create();

    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");
    let webhook_url = format!("{}/notify?sanctifier_provider=discord", server.url());

    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("analyze")
        .arg(fixture_path)
        .arg("--webhook-url")
        .arg(webhook_url)
        .env_remove("RUST_LOG")
        .assert()
        .success()
        .stdout(predicates::str::contains("Static analysis complete."))
        .stderr(predicates::str::contains("Webhook delivery failed"));

    mock.assert();
}

#[test]
fn test_callgraph_generates_dot_for_invoke_contract_calls() {
    let temp_dir = tempdir().unwrap();
    let contract_path = temp_dir.path().join("router.rs");
    let dot_path = temp_dir.path().join("callgraph.dot");

    fs::write(
        &contract_path,
        r#"
            use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

            #[contract]
            pub struct Router;

            #[contractimpl]
            impl Router {
                pub fn forward(env: Env, target: Address, to: Address, amount: i128) {
                    let fn_name = Symbol::new(&env, "transfer");
                    env.invoke_contract::<()>(target, &fn_name, (&to, &amount));
                }
            }
        "#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("callgraph")
        .arg(&contract_path)
        .arg("--output")
        .arg(&dot_path)
        .assert()
        .success();

    let dot = fs::read_to_string(&dot_path).unwrap();
    assert!(dot.contains("digraph ContractCallGraph"));
    assert!(dot.contains("\"Router\" -> \"target\""));
    assert!(dot.contains("fn_name"));
}

#[test]
fn test_analyze_json_includes_call_graph_edges() {
    let temp_dir = tempdir().unwrap();
    let contract_path = temp_dir.path().join("router.rs");

    fs::write(
        &contract_path,
        r#"
            use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

            #[contract]
            pub struct Router;

            #[contractimpl]
            impl Router {
                pub fn forward(env: Env, target: Address, to: Address, amount: i128) {
                    let fn_name = Symbol::new(&env, "transfer");
                    env.invoke_contract::<()>(target, &fn_name, (&to, &amount));
                }
            }
        "#,
    )
    .unwrap();

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(&contract_path)
        .arg("--format")
        .arg("json")
        .env_remove("RUST_LOG")
        .output()
        .expect("sanctifier should run");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be JSON");
    let call_graph = payload["call_graph"]
        .as_array()
        .expect("call_graph should be an array");

    assert_eq!(call_graph.len(), 1);
    assert_eq!(call_graph[0]["caller"], "Router");
    assert_eq!(call_graph[0]["callee"], "target");
    assert_eq!(call_graph[0]["function_expr"], "fn_name");
}
/// Verifies that `sanctifier analyze --format json` output conforms to the
/// published JSON Schema at `schemas/analysis-output.json`.
#[test]
fn test_json_output_validates_against_schema() {
    // Locate the schema relative to the workspace root (two levels up from
    // this package's Cargo.toml directory).
    let schema_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/analysis-output.json");
    let schema_text = fs::read_to_string(&schema_path)
        .expect("schemas/analysis-output.json should exist at the workspace root");
    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_text).expect("schema file should be valid JSON");
    let compiled =
        JSONSchema::compile(&schema_value).expect("schema should compile without errors");

    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/vulnerable_contract.rs");

    let output = Command::cargo_bin("sanctifier")
        .unwrap()
        .arg("analyze")
        .arg(fixture_path)
        .arg("--format")
        .arg("json")
        .env_remove("RUST_LOG")
        .output()
        .expect("sanctifier should run");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let instance: serde_json::Value =
        serde_json::from_str(&stdout).expect("JSON output should parse");

    let result = compiled.validate(&instance);
    if let Err(errors) = result {
        let messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "JSON output does not conform to schemas/analysis-output.json:\n{}",
            messages.join("\n")
        );
    }
}

#[test]
fn test_analyze_with_custom_vuln_db() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let vuln_db_path = env::current_dir()
        .unwrap()
        .join("tests/vulndb/minimal-vulndb.json");
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/vulndb/todo_example.rs");

    cmd.arg("analyze")
        .arg(&fixture_path)
        .arg("--vuln-db")
        .arg(&vuln_db_path)
        .env_remove("RUST_LOG")
        .assert()
        .success();
}

#[test]
fn test_analyze_windows_path_separators() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = if cfg!(windows) {
        "tests\\fixtures\\valid_contract.rs".to_string()
    } else {
        // On Unix, we still want to test that if we pass backslashes,
        // the CLI (or the test environment) can handle it if we've implemented normalization,
        // but for now let's just use the platform-specific path to ensure CI passes.
        // Actually, the requirement said "uses backslashes in --path arg".
        // Let's try to normalize it in the CLI so this test passes on Unix too.
        "tests\\fixtures\\valid_contract.rs".to_string()
    };

    // We need to make sure the file exists at that literal path if we are on Unix and not normalizing.
    // If we ARE normalizing in the CLI, then "tests\\fixtures\\valid_contract.rs" will become "tests/fixtures/valid_contract.rs".

    cmd.arg("analyze")
        .arg(fixture_path)
        .assert()
        .success()
        .stdout(predicates::str::contains("Static analysis complete."));
}

#[test]
fn test_analyze_json_parsable_output() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    let output = cmd
        .arg("analyze")
        .arg(fixture_path)
        .arg("--format")
        .arg("json")
        .env_remove("RUST_LOG")
        .assert()
        .success();

    let stdout_bytes = output.get_output().stdout.clone();
    let stdout = String::from_utf8(stdout_bytes).expect("stdout should be UTF-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("JSON output should be valid JSON");

    assert!(
        parsed["schema_version"].is_string(),
        "JSON should contain schema_version"
    );
    assert!(
        parsed["findings"].is_object(),
        "JSON should contain findings object"
    );
    assert!(
        parsed["metadata"]["project_path"].is_string(),
        "JSON should contain metadata.project_path"
    );
}

#[test]
fn test_analyze_exit_code_on_buggy_fixture() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/vulnerable_contract.rs");

    cmd.arg("analyze")
        .arg(fixture_path)
        .arg("--exit-code")
        .env_remove("RUST_LOG")
        .assert()
        .code(1);
}

#[test]
fn test_analyze_exit_code_on_buggy_fixture_json() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/vulnerable_contract.rs");

    cmd.arg("analyze")
        .arg(fixture_path)
        .arg("--format")
        .arg("json")
        .arg("--exit-code")
        .env_remove("RUST_LOG")
        .assert()
        .code(1);
}

#[test]
fn test_analyze_exit_code_flag_not_set_does_not_fail() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/vulnerable_contract.rs");

    cmd.arg("analyze")
        .arg(fixture_path)
        .env_remove("RUST_LOG")
        .assert()
        .success();
}

#[test]
fn test_analyze_invalid_project_returns_2() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("analyze")
        .arg("does-not-exist")
        .arg("--exit-code")
        .env_remove("RUST_LOG")
        .assert()
        .code(2);
}

#[test]
fn test_init_creates_cargo_toml_and_lib_rs() {
    let temp_dir = tempdir().unwrap();
    let project_path = temp_dir.path().join("test-contract");

    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("init").arg(&project_path).assert().success();

    assert!(
        project_path.join("Cargo.toml").exists(),
        "init should create Cargo.toml"
    );
    assert!(
        project_path.join("src/lib.rs").exists(),
        "init should create src/lib.rs"
    );
}

#[test]
fn test_complexity_shows_table_in_stdout() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    let fixture_path = env::current_dir()
        .unwrap()
        .join("tests/fixtures/valid_contract.rs");

    cmd.arg("complexity")
        .arg(fixture_path)
        .env_remove("RUST_LOG")
        .assert()
        .success()
        .stdout(predicates::str::contains("Function"))
        .stdout(predicates::str::contains("Complexity"));
}
