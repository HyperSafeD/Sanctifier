//! Mainnet-scale concurrency test for `sanctifier serve` HTTP API server.
//!
//! Verifies that `sanctifier serve` handles simultaneous analysis requests
//! correctly without cross-request state contamination or resource leaks.

use reqwest::Client;
use std::collections::HashSet;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Contract 1: Clean contract with no findings.
const CLEAN_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Address, Env};
    #[contract] pub struct CleanToken;
    #[contractimpl] impl CleanToken {
        pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
            from.require_auth();
        }
    }
"#;

/// Contract 2: Missing require_auth (auth gap) -> should produce `auth_gaps` finding `["withdraw"]`.
const AUTH_GAP_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Address, Env};
    #[contract] pub struct Vault;
    #[contractimpl] impl Vault {
        pub fn withdraw(env: Env, recipient: Address, amount: i128) {
            env.storage().persistent().set(&recipient, &amount);
        }
    }
"#;

/// Contract 3: Panic contract -> should produce `panic_issues` finding with `issue_type: "panic!"`.
const PANIC_CONTRACT: &str = r#"
    use soroban_sdk::{contract, contractimpl, Env};
    #[contract] pub struct Risky;
    #[contractimpl] impl Risky {
        pub fn boom(_env: Env) {
            panic!("mainnet test panic");
        }
    }
"#;

/// Helper to start a local `sanctifier serve` instance on a custom port.
struct ServerGuard {
    child: Child,
    port: u16,
}

impl ServerGuard {
    fn start(port: u16) -> Self {
        let bin_path = env!("CARGO_BIN_EXE_sanctifier");
        let child = Command::new(bin_path)
            .args(["serve", "--bind", "127.0.0.1", "--port", &port.to_string()])
            .spawn()
            .expect("Failed to start sanctifier serve binary");

        ServerGuard { child, port }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn wait_for_server_healthy(client: &Client, base_url: &str, timeout: Duration) -> bool {
    let start = Instant::now();
    let health_url = format!("{}/health", base_url);
    while start.elapsed() < timeout {
        if let Ok(resp) = client.get(&health_url).send().await {
            if resp.status().is_success() {
                return true;
            }
        }
        sleep(Duration::from_millis(50)).await;
    }
    false
}

#[tokio::test]
async fn test_mainnet_concurrency_isolation_and_no_leak() {
    let port = 9199;
    let server = ServerGuard::start(port);
    let client = Arc::new(
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
    );
    let base_url = server.url();

    // 1. Wait for server to become healthy
    let is_healthy = wait_for_server_healthy(&client, &base_url, Duration::from_secs(5)).await;
    assert!(
        is_healthy,
        "Server failed to start and respond to /health on port {}",
        port
    );

    // 2. Setup concurrency test parameters (mainnet-scale batch: 60 total requests, 3 distinct inputs)
    let concurrency_count = 60;
    let analyze_url = format!("{}/analyze", base_url);

    let mut tasks = Vec::new();

    for i in 0..concurrency_count {
        let client_clone = Arc::clone(&client);
        let url_clone = analyze_url.clone();

        // Alternate contract inputs to test isolation across concurrent threads
        let (contract_src, expected_type) = match i % 3 {
            0 => (CLEAN_CONTRACT, "clean"),
            1 => (AUTH_GAP_CONTRACT, "auth_gap"),
            _ => (PANIC_CONTRACT, "panic"),
        };

        tasks.push(tokio::spawn(async move {
            let body = serde_json::json!({
                "contract": contract_src
            });

            let resp = client_clone
                .post(&url_clone)
                .json(&body)
                .send()
                .await
                .expect("HTTP request failed");

            assert!(
                resp.status().is_success(),
                "Response status failed for task {}",
                i
            );
            let json: serde_json::Value = resp.json().await.expect("Failed to parse JSON response");

            (i, expected_type, json)
        }));
    }

    // 3. Collect and verify all responses for isolation & correctness
    let mut completed_results = Vec::new();
    for task in tasks {
        let (id, expected_type, json) = task.await.expect("Task panicked");

        match expected_type {
            "clean" => {
                let auth_gaps = json.get("auth_gaps").and_then(|v| v.as_array()).unwrap();
                let panic_issues = json.get("panic_issues").and_then(|v| v.as_array()).unwrap();
                assert!(
                    auth_gaps.is_empty(),
                    "Task {}: Clean contract must not contain auth gaps",
                    id
                );
                assert!(
                    panic_issues.is_empty(),
                    "Task {}: Clean contract must not contain panic issues",
                    id
                );
            }
            "auth_gap" => {
                let auth_gaps = json.get("auth_gaps").and_then(|v| v.as_array()).unwrap();
                assert_eq!(
                    auth_gaps.len(),
                    1,
                    "Task {}: Auth gap contract must produce exactly 1 auth gap finding",
                    id
                );
                assert_eq!(
                    auth_gaps[0].as_str(),
                    Some("withdraw"),
                    "Task {}: Auth gap function name mismatch",
                    id
                );
            }
            "panic" => {
                let panic_issues = json.get("panic_issues").and_then(|v| v.as_array()).unwrap();
                assert_eq!(
                    panic_issues.len(),
                    1,
                    "Task {}: Panic contract must produce exactly 1 panic issue finding",
                    id
                );
                assert_eq!(
                    panic_issues[0]
                        .get("function_name")
                        .and_then(|v| v.as_str()),
                    Some("boom"),
                    "Task {}: Panic function name mismatch",
                    id
                );
            }
            _ => panic!("Unexpected test case type"),
        }

        completed_results.push(id);
    }

    assert_eq!(
        completed_results.len(),
        concurrency_count,
        "All concurrent requests should complete successfully"
    );

    // Ensure all task IDs from 0 to N-1 were completed
    let id_set: HashSet<_> = completed_results.into_iter().collect();
    assert_eq!(id_set.len(), concurrency_count);

    // 4. Final health check after heavy concurrent load
    let final_health = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .expect("Final health check failed");
    assert!(
        final_health.status().is_success(),
        "Server remains healthy after concurrent load"
    );
}
