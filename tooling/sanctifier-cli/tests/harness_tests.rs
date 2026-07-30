/// Integration / e2e tests for `sanctifier harness` (#415).
///
/// Covers: default (both backends), single-backend selection, function
/// filtering, generated file/manifest shape, crate auto-detection, the
/// "nothing to fuzz" case, and a nonexistent source path.
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn write_contract(dir: &tempfile::TempDir, name: &str, src: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, src).unwrap();
    path
}

const TOKEN_SRC: &str = r#"
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {}
    pub fn balance(env: Env, id: Address) -> i128 { 0 }
}
"#;

// ── default (both backends) ────────────────────────────────────────────────

#[test]
fn harness_default_generates_both_backends() {
    let dir = tempdir().unwrap();
    let src = write_contract(&dir, "token.rs", TOKEN_SRC);
    let output = dir.path().join("fuzz-out");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["harness"])
        .arg(&src)
        .arg("--output")
        .arg(&output)
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated"))
        .stdout(predicate::str::contains("4 fuzz target(s)"));

    assert!(output.join("afl").join("Cargo.toml").is_file());
    assert!(output.join("honggfuzz").join("Cargo.toml").is_file());
    assert!(output
        .join("afl")
        .join("src")
        .join("bin")
        .join("token_transfer.rs")
        .is_file());
    assert!(output
        .join("afl")
        .join("src")
        .join("bin")
        .join("token_balance.rs")
        .is_file());
    assert!(output
        .join("honggfuzz")
        .join("src")
        .join("bin")
        .join("token_transfer.rs")
        .is_file());
}

// ── single-backend selection ────────────────────────────────────────────────

#[test]
fn harness_target_afl_only_generates_afl_directory() {
    let dir = tempdir().unwrap();
    let src = write_contract(&dir, "token.rs", TOKEN_SRC);
    let output = dir.path().join("fuzz-out");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["harness"])
        .arg(&src)
        .arg("--output")
        .arg(&output)
        .args(["--target", "afl"])
        .assert()
        .success();

    assert!(output.join("afl").join("Cargo.toml").is_file());
    assert!(
        !output.join("honggfuzz").exists(),
        "honggfuzz dir must not be generated when --target afl is given"
    );
}

#[test]
fn harness_target_honggfuzz_only_generates_honggfuzz_directory() {
    let dir = tempdir().unwrap();
    let src = write_contract(&dir, "token.rs", TOKEN_SRC);
    let output = dir.path().join("fuzz-out");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["harness"])
        .arg(&src)
        .arg("--output")
        .arg(&output)
        .args(["--target", "honggfuzz"])
        .assert()
        .success();

    assert!(output.join("honggfuzz").join("Cargo.toml").is_file());
    assert!(!output.join("afl").exists());
}

// ── function filtering ──────────────────────────────────────────────────────

#[test]
fn harness_function_filter_restricts_to_one_function() {
    let dir = tempdir().unwrap();
    let src = write_contract(&dir, "token.rs", TOKEN_SRC);
    let output = dir.path().join("fuzz-out");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["harness"])
        .arg(&src)
        .arg("--output")
        .arg(&output)
        .args(["--target", "afl", "--function", "transfer"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 fuzz target(s)"));

    assert!(output
        .join("afl")
        .join("src")
        .join("bin")
        .join("token_transfer.rs")
        .is_file());
    assert!(!output
        .join("afl")
        .join("src")
        .join("bin")
        .join("token_balance.rs")
        .exists());
}

// ── generated content shape ─────────────────────────────────────────────────

#[test]
fn harness_generated_afl_source_has_expected_shape() {
    let dir = tempdir().unwrap();
    let src = write_contract(&dir, "token.rs", TOKEN_SRC);
    let output = dir.path().join("fuzz-out");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["harness"])
        .arg(&src)
        .arg("--output")
        .arg(&output)
        .args(["--target", "afl", "--function", "transfer"])
        .assert()
        .success();

    let content = fs::read_to_string(
        output
            .join("afl")
            .join("src")
            .join("bin")
            .join("token_transfer.rs"),
    )
    .unwrap();

    assert!(content.contains("use afl::fuzz;"));
    assert!(content.contains("#[derive(Debug, Arbitrary)]"));
    assert!(content.contains("struct FuzzInput {"));
    assert!(content.contains("from: <Address as SorobanArbitrary>::Prototype,"));
    assert!(content.contains("amount: <i128 as SorobanArbitrary>::Prototype,"));
    assert!(content.contains("let contract_id = env.register_contract(None, Token);"));
    assert!(content.contains("let client = TokenClient::new(&env, &contract_id);"));
    assert!(content.contains("let _ = client.try_transfer(&from, &to, &amount);"));
    // The mandatory leading `Env` parameter must never appear as a fuzzed field.
    assert!(!content.contains("env: <Env"));
}

#[test]
fn harness_generated_manifest_excludes_parent_workspace() {
    let dir = tempdir().unwrap();
    let src = write_contract(&dir, "token.rs", TOKEN_SRC);
    let output = dir.path().join("fuzz-out");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["harness"])
        .arg(&src)
        .arg("--output")
        .arg(&output)
        .args(["--target", "afl"])
        .assert()
        .success();

    let manifest = fs::read_to_string(output.join("afl").join("Cargo.toml")).unwrap();
    assert!(manifest.contains("[workspace]"));
    assert!(manifest.contains("afl = \"0.15\""));
    assert!(manifest.contains("[[bin]]"));
    assert!(manifest.contains("soroban-sdk"));
    assert!(manifest.contains("testutils"));
}

// ── crate auto-detection ────────────────────────────────────────────────────

#[test]
fn harness_detects_sibling_cargo_toml_and_adds_path_dependency() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"my-token\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let src = src_dir.join("lib.rs");
    fs::write(&src, TOKEN_SRC).unwrap();
    let output = dir.path().join("fuzz-out");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["harness"])
        .arg(&src)
        .arg("--output")
        .arg(&output)
        .args(["--target", "afl"])
        .assert()
        .success();

    let manifest = fs::read_to_string(output.join("afl").join("Cargo.toml")).unwrap();
    assert!(
        manifest.contains("my-token = { path"),
        "manifest should reference the detected crate by name:\n{manifest}"
    );

    let source = fs::read_to_string(
        output
            .join("afl")
            .join("src")
            .join("bin")
            .join("token_transfer.rs"),
    )
    .unwrap();
    assert!(source.contains("use my_token::{Token, TokenClient};"));
}

// ── nothing to fuzz ──────────────────────────────────────────────────────────

#[test]
fn harness_contract_with_no_public_functions_warns_and_generates_nothing() {
    let dir = tempdir().unwrap();
    let src = write_contract(
        &dir,
        "empty.rs",
        "pub struct MyContract;\nimpl MyContract {}",
    );
    let output = dir.path().join("fuzz-out");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["harness"])
        .arg(&src)
        .arg("--output")
        .arg(&output)
        .assert()
        .success()
        .stdout(predicate::str::contains("No fuzzable public contract functions"));

    assert!(!output.exists(), "no output directory should be created");
}

// ── error cases ──────────────────────────────────────────────────────────────

#[test]
fn harness_nonexistent_path_exits_with_error() {
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["harness", "/no/such/file.rs"])
        .assert()
        .failure();
}
