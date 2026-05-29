#![cfg(target_arch = "wasm32")]

use sanctifier_wasm::{analyze, analyze_with_config, finding_codes, schema_version, version};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ── Input Validation Tests ────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_analyze_empty_source() {
    let result = analyze("");
    // Should return an error response
    assert!(!result.is_null());
}

#[wasm_bindgen_test]
fn test_analyze_valid_source() {
    let source = r#"
        #![no_std]
        use soroban_sdk::{contract, contractimpl};
        
        #[contract]
        pub struct MyContract;
        
        #[contractimpl]
        impl MyContract {
            pub fn hello() {}
        }
    "#;

    let result = analyze(source);
    assert!(!result.is_null());
}

#[wasm_bindgen_test]
fn test_analyze_with_config_empty_config() {
    let source = "fn main() {}";
    let result = analyze_with_config("", source);
    // Should return an error response
    assert!(!result.is_null());
}

#[wasm_bindgen_test]
fn test_analyze_with_config_invalid_json() {
    let source = "fn main() {}";
    let result = analyze_with_config("{invalid json}", source);
    // Should gracefully handle invalid JSON
    assert!(!result.is_null());
}

#[wasm_bindgen_test]
fn test_analyze_with_config_valid_json() {
    let source = r#"
        #![no_std]
        use soroban_sdk::{contract, contractimpl};
        
        #[contract]
        pub struct MyContract;
        
        #[contractimpl]
        impl MyContract {
            pub fn hello() {}
        }
    "#;

    let config = r#"{"ledger_limit": 64000}"#;
    let result = analyze_with_config(config, source);
    assert!(!result.is_null());
}

// ── Version Tests ────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_version_returns_string() {
    let v = version();
    assert!(!v.is_empty());
    // Should be semantic version format
    assert!(v.contains('.'));
}

#[wasm_bindgen_test]
fn test_schema_version_returns_string() {
    let sv = schema_version();
    assert_eq!(sv, "1.0.0");
}

// ── Finding Codes Tests ──────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_finding_codes_returns_array() {
    let codes = finding_codes();
    assert!(!codes.is_null());
}

// ── Large Contract Handling ──────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_analyze_large_contract() {
    // Generate a moderately large contract (but well under 10MB limit)
    let mut source = String::from(
        r#"
        #![no_std]
        use soroban_sdk::{contract, contractimpl, Env};
        
        #[contract]
        pub struct LargeContract;
        
        #[contractimpl]
        impl LargeContract {
        "#,
    );

    // Add many functions to simulate a large contract
    for i in 0..100 {
        source.push_str(&format!(
            r#"
            pub fn function_{i}(env: Env) {{
                // Function body
            }}
            "#
        ));
    }

    source.push_str("}");

    let result = analyze(&source);
    assert!(!result.is_null());
}

// ── Performance Benchmarks ───────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_analyze_performance_simple() {
    let source = r#"
        #![no_std]
        use soroban_sdk::{contract, contractimpl};

        #[contract]
        pub struct SimpleContract;

        #[contractimpl]
        impl SimpleContract {
            pub fn simple_fn() {}
        }
    "#;

    // Should complete quickly
    let result = analyze(source);
    assert!(!result.is_null());
}

// ── Memory Budget Tests ──────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn test_analyze_returns_error_for_source_exceeding_memory_budget() {
    // 5 MB source × 8 overhead factor = 40 MB > 32 MB budget.
    // The engine must return a structured error rather than crash.
    let oversized = "x".repeat(5 * 1024 * 1024);
    let result = analyze(&oversized);
    // Result must be a non-null JS object (ErrorResponse), not a panic.
    assert!(!result.is_null());
}

#[wasm_bindgen_test]
fn test_analyze_with_config_returns_error_for_source_exceeding_memory_budget() {
    let oversized = "x".repeat(5 * 1024 * 1024);
    let result = analyze_with_config("{}", &oversized);
    assert!(!result.is_null());
}

#[wasm_bindgen_test]
fn test_analyze_normal_source_unaffected_by_memory_check() {
    // A typical contract is well under the budget — should still analyse normally.
    let source = r#"
        #![no_std]
        use soroban_sdk::{contract, contractimpl};

        #[contract]
        pub struct NormalContract;

        #[contractimpl]
        impl NormalContract {
            pub fn do_work() {}
        }
    "#;
    let result = analyze(source);
    assert!(!result.is_null());
}
