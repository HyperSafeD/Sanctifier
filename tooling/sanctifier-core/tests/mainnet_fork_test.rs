//! Read-only mainnet fork integration tests.
//!
//! These tests fetch real, publicly-deployed Soroban contracts from Stellar
//! mainnet (via the Stellar Expert public API) and run the full
//! `sanctifier-core` analysis engine against them **without submitting any
//! transaction**.  The purpose is to validate that the engine does not crash
//! or hang on real production code patterns that synthetic fixtures don't
//! exercise (macro usage, complex generics, large codebases, multi-hop call
//! chains, etc.).
//!
//! # Opt-in guard
//!
//! The tests are **skipped by default** so that `cargo test` in a local
//! dev environment never makes unexpected network calls.  Set the environment
//! variable `SANCTIFIER_MAINNET_FORK=1` to enable them:
//!
//! ```bash
//! SANCTIFIER_MAINNET_FORK=1 cargo test --test mainnet_fork_test -p sanctifier-core -- --nocapture
//! ```
//!
//! To run a single corpus entry:
//!
//! ```bash
//! SANCTIFIER_MAINNET_FORK=1 SANCTIFIER_FORK_CONTRACT=soroswap-router \
//!   cargo test --test mainnet_fork_test -p sanctifier-core -- --nocapture
//! ```
//!
//! # What is validated
//!
//! For every contract in `tests/mainnet-fork/corpus.json`:
//!
//! 1. The contract source (Rust) is fetched from Stellar Expert's public API.
//! 2. `RuleRegistry::run_all` is called — must not panic, must not hang
//!    (enforced by a 120-second thread-based timeout).
//! 3. Every returned `RuleViolation` must reference a valid known rule name.
//! 4. Results are written to `target/mainnet-fork-report.json` for CI upload.
//!
//! # Out of scope
//!
//! - No state mutation.
//! - No transaction submission.
//! - No wallet or key material required.

use sanctifier_core::rules::{RuleRegistry, RuleViolation};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::Write,
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Environment variable that must be set to `"1"` to opt into network calls.
const OPT_IN_ENV: &str = "SANCTIFIER_MAINNET_FORK";

/// Optional env var to restrict the run to a single corpus ID.
const SINGLE_CONTRACT_ENV: &str = "SANCTIFIER_FORK_CONTRACT";

/// Per-contract analysis timeout.
const ANALYSIS_TIMEOUT: Duration = Duration::from_secs(120);

/// Stellar Expert contract source endpoint (returns Rust source when available).
const STELLAR_EXPERT_SOURCE_BASE: &str = "https://api.stellar.expert/explorer/public/contract";

/// Corpus manifest path relative to workspace root.
const CORPUS_MANIFEST: &str = "tests/mainnet-fork/corpus.json";

/// Output report path relative to workspace root.
const REPORT_OUTPUT: &str = "target/mainnet-fork-report.json";

// ── Corpus schema ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CorpusManifest {
    corpus: Vec<CorpusEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct CorpusEntry {
    id: String,
    name: String,
    contract_id: String,
    description: String,
    #[serde(default)]
    expected_findings: Vec<String>,
    #[serde(default)]
    skip_if_unreachable: bool,
}

// ── Report schema ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ForkReport {
    schema_version: &'static str,
    generated_at: String,
    total_contracts: usize,
    passed: usize,
    skipped: usize,
    failed: usize,
    results: Vec<ContractResult>,
}

#[derive(Debug, Serialize)]
struct ContractResult {
    id: String,
    name: String,
    contract_id: String,
    status: ResultStatus,
    findings_count: usize,
    unexpected_findings: Vec<String>,
    duration_ms: u64,
    error: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ResultStatus {
    Passed,
    Skipped,
    Failed,
}

// ── Fetch helpers ─────────────────────────────────────────────────────────────

/// Attempts to fetch the Rust source for a contract from Stellar Expert.
/// Returns `None` if the contract has no indexed source or the request fails.
fn fetch_contract_source(contract_id: &str) -> Option<String> {
    let url = format!("{}/{}/source", STELLAR_EXPERT_SOURCE_BASE, contract_id);

    // Use curl via std::process to avoid adding an async HTTP dep.
    let output = std::process::Command::new("curl")
        .args([
            "--silent",
            "--fail",
            "--max-time",
            "30",
            "--user-agent",
            "sanctifier-mainnet-fork-test/1.0",
            &url,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // Stellar Expert returns a JSON envelope; extract the `source` field.
    let body = String::from_utf8(output.stdout).ok()?;
    parse_source_from_response(&body)
}

/// Parses the Rust source string out of the Stellar Expert JSON response.
///
/// Expected shape:
/// ```json
/// { "source": { "lib.rs": "pub fn ..." } }
/// ```
fn parse_source_from_response(body: &str) -> Option<String> {
    // Minimal extraction — avoids pulling in a full JSON dep for a test binary.
    // We look for the first `"lib.rs":` key and grab its string value.
    let key = "\"lib.rs\":";
    let start = body.find(key)? + key.len();
    let rest = body[start..].trim();
    if !rest.starts_with('"') {
        return None;
    }
    // The value is a JSON string — find the closing quote (handling escapes).
    let inner = &rest[1..];
    let mut result = String::new();
    let mut chars = inner.chars().peekable();
    loop {
        match chars.next()? {
            '"' => break,
            '\\' => match chars.next()? {
                'n' => result.push('\n'),
                't' => result.push('\t'),
                'r' => result.push('\r'),
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                c => {
                    result.push('\\');
                    result.push(c);
                }
            },
            c => result.push(c),
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

// ── Analysis with timeout ─────────────────────────────────────────────────────

/// Runs `RuleRegistry::run_all` on `source` with a hard timeout.
///
/// Returns `Ok(violations)` on success, `Err(reason)` on timeout or panic.
fn analyze_with_timeout(source: String) -> Result<Vec<RuleViolation>, String> {
    let (tx, rx) = mpsc::channel();
    let source_clone = source.clone();

    thread::spawn(move || {
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let registry = RuleRegistry::with_default_rules();
            registry.run_all(&source_clone)
        }));
        let _ = tx.send(result);
    });

    match rx.recv_timeout(ANALYSIS_TIMEOUT) {
        Ok(Ok(violations)) => Ok(violations),
        Ok(Err(panic_payload)) => {
            let msg = panic_payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic_payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".to_string());
            Err(format!("analysis panicked: {msg}"))
        }
        Err(_) => Err(format!(
            "analysis timed out after {}s",
            ANALYSIS_TIMEOUT.as_secs()
        )),
    }
}

// ── Report writer ─────────────────────────────────────────────────────────────

fn write_report(report: &ForkReport, workspace_root: &Path) {
    let path = workspace_root.join(REPORT_OUTPUT);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(report).unwrap_or_default();
    if let Ok(mut f) = fs::File::create(&path) {
        let _ = f.write_all(json.as_bytes());
    }
    println!("Fork test report written to: {}", path.display());
}

// ── Main test ─────────────────────────────────────────────────────────────────

#[test]
fn mainnet_fork_read_only_analysis() {
    // ── Opt-in guard ─────────────────────────────────────────────────────────
    if env::var(OPT_IN_ENV).as_deref() != Ok("1") {
        println!(
            "Mainnet fork tests skipped. Set {}=1 to enable.",
            OPT_IN_ENV
        );
        return;
    }

    // ── Locate workspace root ─────────────────────────────────────────────────
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // sanctifier-core lives at tooling/sanctifier-core; workspace root is two levels up.
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("could not resolve workspace root from CARGO_MANIFEST_DIR")
        .to_path_buf();

    // ── Load corpus ───────────────────────────────────────────────────────────
    let corpus_path = workspace_root.join(CORPUS_MANIFEST);
    let corpus_json =
        fs::read_to_string(&corpus_path).expect("could not read tests/mainnet-fork/corpus.json");
    let manifest: CorpusManifest =
        serde_json::from_str(&corpus_json).expect("corpus.json is not valid JSON");

    // Optional: restrict to a single contract ID via env var.
    let filter = env::var(SINGLE_CONTRACT_ENV).ok();

    let registry = RuleRegistry::with_default_rules();
    let known_rules: std::collections::HashSet<String> = registry
        .available_rules()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut results: Vec<ContractResult> = Vec::new();
    let mut hard_failures: Vec<String> = Vec::new();

    for entry in &manifest.corpus {
        // Apply single-contract filter if set.
        if let Some(ref id) = filter {
            if &entry.id != id {
                continue;
            }
        }

        println!("\n─── {} ({}) ───", entry.name, entry.id);
        println!("    contract_id : {}", entry.contract_id);
        println!("    description : {}", entry.description);

        let start = Instant::now();

        // ── Fetch source ──────────────────────────────────────────────────────
        let source = match fetch_contract_source(&entry.contract_id) {
            Some(s) => s,
            None => {
                println!("    [SKIP] Could not fetch source (network error or no indexed source).");
                if !entry.skip_if_unreachable {
                    hard_failures.push(format!(
                        "{}: fetch failed and skip_if_unreachable is false",
                        entry.id
                    ));
                }
                results.push(ContractResult {
                    id: entry.id.clone(),
                    name: entry.name.clone(),
                    contract_id: entry.contract_id.clone(),
                    status: ResultStatus::Skipped,
                    findings_count: 0,
                    unexpected_findings: vec![],
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some("source not available".to_string()),
                });
                continue;
            }
        };

        println!("    source size : {} bytes", source.len());

        // ── Analyse ───────────────────────────────────────────────────────────
        let analysis_result = analyze_with_timeout(source);
        let duration_ms = start.elapsed().as_millis() as u64;

        match analysis_result {
            Ok(violations) => {
                // Validate all rule names are known.
                let mut bad_rules: Vec<String> = vec![];
                for v in &violations {
                    if !known_rules.contains(&v.rule_name) {
                        bad_rules.push(v.rule_name.clone());
                    }
                }

                // Check for unexpected findings (warn, not fail).
                let unexpected: Vec<String> = violations
                    .iter()
                    .filter(|v| !entry.expected_findings.contains(&v.rule_name))
                    .map(|v| format!("{}@{}", v.rule_name, v.location))
                    .collect();

                if !unexpected.is_empty() {
                    println!(
                        "    [WARN] {} unexpected finding(s) — manual triage required:",
                        unexpected.len()
                    );
                    for u in &unexpected {
                        println!("           {u}");
                    }
                }

                if !bad_rules.is_empty() {
                    let msg = format!(
                        "{}: violations reference unknown rule names: {:?}",
                        entry.id, bad_rules
                    );
                    hard_failures.push(msg.clone());
                    println!("    [FAIL] {msg}");
                    results.push(ContractResult {
                        id: entry.id.clone(),
                        name: entry.name.clone(),
                        contract_id: entry.contract_id.clone(),
                        status: ResultStatus::Failed,
                        findings_count: violations.len(),
                        unexpected_findings: unexpected,
                        duration_ms,
                        error: Some(msg),
                    });
                } else {
                    println!(
                        "    [PASS] {} finding(s) in {}ms",
                        violations.len(),
                        duration_ms
                    );
                    results.push(ContractResult {
                        id: entry.id.clone(),
                        name: entry.name.clone(),
                        contract_id: entry.contract_id.clone(),
                        status: ResultStatus::Passed,
                        findings_count: violations.len(),
                        unexpected_findings: unexpected,
                        duration_ms,
                        error: None,
                    });
                }
            }
            Err(reason) => {
                let msg = format!("{}: {}", entry.id, reason);
                hard_failures.push(msg.clone());
                println!("    [FAIL] {reason}");
                results.push(ContractResult {
                    id: entry.id.clone(),
                    name: entry.name.clone(),
                    contract_id: entry.contract_id.clone(),
                    status: ResultStatus::Failed,
                    findings_count: 0,
                    unexpected_findings: vec![],
                    duration_ms,
                    error: Some(reason),
                });
            }
        }
    }

    // ── Write report ──────────────────────────────────────────────────────────
    let passed = results
        .iter()
        .filter(|r| r.status == ResultStatus::Passed)
        .count();
    let skipped = results
        .iter()
        .filter(|r| r.status == ResultStatus::Skipped)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.status == ResultStatus::Failed)
        .count();

    let report = ForkReport {
        schema_version: "1",
        generated_at: chrono::Utc::now().to_rfc3339(),
        total_contracts: results.len(),
        passed,
        skipped,
        failed,
        results,
    };

    write_report(&report, &workspace_root);

    println!("\n═══ Mainnet Fork Summary ═══");
    println!("  passed  : {passed}");
    println!("  skipped : {skipped}");
    println!("  failed  : {failed}");

    // ── Assert no hard failures ───────────────────────────────────────────────
    assert!(
        hard_failures.is_empty(),
        "Mainnet fork test failures:\n{}",
        hard_failures
            .iter()
            .map(|s| format!("  • {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ── Unit tests for fetch/parse helpers ────────────────────────────────────────

#[test]
fn parse_source_from_stellar_expert_response_extracts_lib_rs() {
    let response = r#"{"source":{"lib.rs":"pub fn hello() {}"}}"#;
    let src = parse_source_from_response(response);
    assert_eq!(src, Some("pub fn hello() {}".to_string()));
}

#[test]
fn parse_source_handles_escape_sequences() {
    let response = r#"{"source":{"lib.rs":"fn f() {\n    let x = 1;\n}"}}"#;
    let src = parse_source_from_response(response);
    assert_eq!(src, Some("fn f() {\n    let x = 1;\n}".to_string()));
}

#[test]
fn parse_source_returns_none_for_missing_lib_rs() {
    let response = r#"{"source":{"other.rs":"pub fn foo() {}"}}"#;
    assert!(parse_source_from_response(response).is_none());
}

#[test]
fn parse_source_returns_none_for_empty_body() {
    assert!(parse_source_from_response("{}").is_none());
}

#[test]
fn analyze_with_timeout_does_not_panic_on_empty_source() {
    let result = analyze_with_timeout(String::new());
    assert!(result.is_ok(), "empty source must not crash or timeout");
    assert!(result.unwrap().is_empty());
}

#[test]
fn analyze_with_timeout_does_not_panic_on_valid_contract() {
    let source = r#"
        use soroban_sdk::{contract, contractimpl, Address, Env};
        #[contract] pub struct Token;
        #[contractimpl] impl Token {
            pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                from.require_auth();
            }
        }
    "#;
    let result = analyze_with_timeout(source.to_string());
    assert!(
        result.is_ok(),
        "valid contract must not crash: {:?}",
        result
    );
}

#[test]
fn analyze_with_timeout_surfaces_findings_for_auth_gap() {
    let source = r#"
        use soroban_sdk::{contract, contractimpl, Address, Env};
        #[contract] pub struct Vault;
        #[contractimpl] impl Vault {
            pub fn withdraw(env: Env, recipient: Address, amount: i128) {
                env.storage().persistent().set(&recipient, &amount);
            }
        }
    "#;
    let result = analyze_with_timeout(source.to_string());
    assert!(result.is_ok());
    let violations = result.unwrap();
    assert!(
        !violations.is_empty(),
        "auth-gap contract must produce at least one violation"
    );
}
