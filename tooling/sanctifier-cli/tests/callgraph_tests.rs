/// Integration / e2e tests for `sanctifier callgraph` (#526).
///
/// Covers: empty contract, invoke_contract_check variant, multiple callers,
/// nonexistent path, default output file, and DOT syntax correctness.
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

// ── helpers ──────────────────────────────────────────────────────────────────

fn write_contract(dir: &tempfile::TempDir, name: &str, src: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, src).unwrap();
    path
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// A contract with no `invoke_contract` calls produces a graph with 0 edges.
#[test]
fn callgraph_empty_contract_has_no_edges() {
    let dir = tempdir().unwrap();
    let src = write_contract(
        &dir,
        "empty.rs",
        "pub struct MyContract;\nimpl MyContract {}",
    );
    let dot = dir.path().join("out.dot");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["callgraph"])
        .arg(&src)
        .arg("--output")
        .arg(&dot)
        .assert()
        .success()
        .stdout(predicate::str::contains("0 edge(s)"));

    let content = fs::read_to_string(&dot).unwrap();
    assert!(
        content.contains("digraph ContractCallGraph"),
        "must have digraph header"
    );
    assert!(
        !content.contains(" -> "),
        "empty contract must not produce edges"
    );
}

/// `invoke_contract_check` is a second call variant that must also be detected.
#[test]
fn callgraph_detects_invoke_contract_check_variant() {
    let dir = tempdir().unwrap();
    let src = write_contract(
        &dir,
        "checked.rs",
        r#"
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct Checker;

#[contractimpl]
impl Checker {
    pub fn safe_call(env: Env, target: Address) {
        let _result = env.invoke_contract_check::<()>(target, &Symbol::new(&env, "ping"), ());
    }
}
"#,
    );
    let dot = dir.path().join("checked.dot");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["callgraph"])
        .arg(&src)
        .arg("--output")
        .arg(&dot)
        .assert()
        .success()
        .stdout(predicate::str::contains("1 edge(s)"));

    let content = fs::read_to_string(&dot).unwrap();
    assert!(content.contains("\"Checker\" -> \"target\""));
}

/// Two functions in the same contract each calling `invoke_contract` produces
/// two separate edges.
#[test]
fn callgraph_multiple_callers_in_one_contract_produce_multiple_edges() {
    let dir = tempdir().unwrap();
    let src = write_contract(
        &dir,
        "multi.rs",
        r#"
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct Hub;

#[contractimpl]
impl Hub {
    pub fn call_a(env: Env, target: Address) {
        env.invoke_contract::<()>(target, &Symbol::new(&env, "fn_a"), ());
    }
    pub fn call_b(env: Env, target: Address) {
        env.invoke_contract::<()>(target, &Symbol::new(&env, "fn_b"), ());
    }
}
"#,
    );
    let dot = dir.path().join("multi.dot");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["callgraph"])
        .arg(&src)
        .arg("--output")
        .arg(&dot)
        .assert()
        .success()
        .stdout(predicate::str::contains("2 edge(s)"));

    let content = fs::read_to_string(&dot).unwrap();
    assert_eq!(
        content.matches("\"Hub\" -> \"target\"").count(),
        2,
        "both callers must appear as edges"
    );
}

/// A nonexistent source path must cause a non-zero exit.
#[test]
fn callgraph_nonexistent_path_exits_with_error() {
    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["callgraph", "/no/such/file.rs"])
        .assert()
        .failure();
}

/// Without `--output`, the DOT file is written to `callgraph.dot` in the
/// current working directory.
#[test]
fn callgraph_default_output_is_callgraph_dot_in_cwd() {
    let dir = tempdir().unwrap();
    let src = write_contract(
        &dir,
        "contract.rs",
        r#"
#[contractimpl]
impl MyContract {
    pub fn noop(_env: Env) {}
}
"#,
    );

    Command::cargo_bin("sanctifier")
        .unwrap()
        .current_dir(dir.path())
        .args(["callgraph"])
        .arg(&src)
        .assert()
        .success();

    assert!(
        dir.path().join("callgraph.dot").exists(),
        "default output file callgraph.dot must be created in cwd"
    );
}

/// The generated DOT file must have valid DOT syntax: starts with `digraph`,
/// closes with `}`, and edges carry `[label=…]` attributes.
#[test]
fn callgraph_dot_output_is_structurally_valid() {
    let dir = tempdir().unwrap();
    let src = write_contract(
        &dir,
        "router.rs",
        r#"
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct Router;

#[contractimpl]
impl Router {
    pub fn route(env: Env, target: Address) {
        env.invoke_contract::<()>(target, &Symbol::new(&env, "execute"), ());
    }
}
"#,
    );
    let dot = dir.path().join("router.dot");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["callgraph"])
        .arg(&src)
        .arg("--output")
        .arg(&dot)
        .assert()
        .success();

    let content = fs::read_to_string(&dot).unwrap();
    assert!(
        content.starts_with("digraph"),
        "DOT file must start with 'digraph'"
    );
    assert!(
        content.trim_end().ends_with('}'),
        "DOT file must end with closing brace"
    );
    assert!(
        content.contains("[label="),
        "edges must carry label attributes"
    );
}

/// Two separate impl blocks (two contracts) in one file each produce edges
/// attributed to their respective contract name.
#[test]
fn callgraph_two_contracts_in_one_file_are_attributed_correctly() {
    let dir = tempdir().unwrap();
    let src = write_contract(
        &dir,
        "two_contracts.rs",
        r#"
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct Alpha;

#[contractimpl]
impl Alpha {
    pub fn go(env: Env, target: Address) {
        env.invoke_contract::<()>(target, &Symbol::new(&env, "alpha_fn"), ());
    }
}

#[contract]
pub struct Beta;

#[contractimpl]
impl Beta {
    pub fn go(env: Env, target: Address) {
        env.invoke_contract::<()>(target, &Symbol::new(&env, "beta_fn"), ());
    }
}
"#,
    );
    let dot = dir.path().join("two.dot");

    Command::cargo_bin("sanctifier")
        .unwrap()
        .args(["callgraph"])
        .arg(&src)
        .arg("--output")
        .arg(&dot)
        .assert()
        .success()
        .stdout(predicate::str::contains("2 edge(s)"));

    let content = fs::read_to_string(&dot).unwrap();
    assert!(
        content.contains("\"Alpha\" -> \"target\""),
        "Alpha edge missing"
    );
    assert!(
        content.contains("\"Beta\" -> \"target\""),
        "Beta edge missing"
    );
}
