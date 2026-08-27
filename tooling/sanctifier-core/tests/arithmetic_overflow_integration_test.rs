//! Integration tests for S003 (arithmetic_overflow) rule.
//!
//! These tests run the CLI against mock contracts and assert on the SARIF/JSON output.
//! This validates end-to-end behavior including:
//! - CLI argument parsing and contract loading
//! - Rule execution in the analysis engine
//! - SARIF report generation with correct severity/location
//! - JSON output formatting
//!
//! # Test Organization
//!
//! - `test_cli_detects_arithmetic_overflow` - Basic CLI detection test
//! - `test_sarif_output_structure` - Validates SARIF schema compliance
//! - `test_json_output_contains_violations` - Validates JSON format
//! - `test_cli_exit_code_with_findings` - Validates CLI exit behavior
//! - `test_multiple_findings_deduplication` - Validates per-function deduplication
//! - `test_safe_arithmetic_no_findings` - Validates true negatives
//!
//! # Running Tests
//!
//! ```bash
//! cargo test --test arithmetic_overflow_integration_test
//! ```

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to create a temporary test workspace with a contract file.
///
/// Returns (temp_dir, contract_path) tuple. The temp_dir must be kept alive
/// for the duration of the test to prevent cleanup.
fn create_test_workspace(contract_content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let contract_path = temp_dir.path().join("lib.rs");
    fs::write(&contract_path, contract_content).expect("Failed to write contract");
    (temp_dir, contract_path)
}

/// Helper to run the Sanctifier CLI and capture output.
///
/// # Arguments
///
/// - `contract_path` - Path to the contract file to analyze
/// - `format` - Output format ("json" or "sarif")
///
/// # Returns
///
/// Tuple of (exit_code, stdout, stderr)
fn run_cli(contract_path: &PathBuf, format: &str) -> (i32, String, String) {
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "sanctifier",
            "--",
            "analyze",
            contract_path.to_str().unwrap(),
            "--format",
            format,
        ])
        .output()
        .expect("Failed to execute CLI");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

#[test]
fn test_cli_detects_arithmetic_overflow() {
    let contract = r#"
        pub fn transfer(amount: u64, fee: u64) -> u64 {
            let total = amount + fee;  // Should trigger S003
            total
        }

        pub fn mint(balance: i128, new_amount: i128) -> i128 {
            balance + new_amount  // Should trigger S003
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(contract);
    let (exit_code, stdout, _stderr) = run_cli(&contract_path, "json");

    // CLI should exit with non-zero when findings are present
    assert_ne!(exit_code, 0, "CLI should return non-zero exit code with findings");

    // Parse JSON output
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse JSON output");

    // Extract findings array
    let findings = json["findings"].as_array()
        .expect("Output should contain 'findings' array");

    // Should have 2 findings (one per function)
    assert_eq!(findings.len(), 2, "Should detect 2 arithmetic overflow issues");

    // Verify finding structure
    for finding in findings {
        assert_eq!(finding["rule"].as_str().unwrap(), "arithmetic_overflow");
        assert_eq!(finding["severity"].as_str().unwrap(), "warning");
        assert!(finding["message"].as_str().unwrap().contains("overflow"));
        assert!(finding["location"].as_str().is_some());
    }
}

#[test]
fn test_sarif_output_structure() {
    let contract = r#"
        pub fn calculate(a: u64, b: u64) -> u64 {
            let sum = a + b;      // S003
            let product = a * b;  // S003
            sum + product
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(contract);
    let (_exit_code, stdout, _stderr) = run_cli(&contract_path, "sarif");

    // Parse SARIF output
    let sarif: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse SARIF output");

    // Validate SARIF structure
    assert_eq!(sarif["version"].as_str().unwrap(), "2.1.0");
    assert_eq!(sarif["$schema"].as_str().unwrap(), 
        "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json");

    let runs = sarif["runs"].as_array()
        .expect("SARIF should contain 'runs' array");
    assert!(!runs.is_empty(), "SARIF runs array should not be empty");

    let run = &runs[0];
    assert!(run["tool"]["driver"]["name"].as_str().is_some());
    
    let results = run["results"].as_array()
        .expect("SARIF run should contain 'results' array");
    
    // Should have 2 findings (deduped per function: + and *)
    assert_eq!(results.len(), 2, "Should have 2 deduplicated findings");

    // Validate each result structure
    for result in results {
        assert!(result["ruleId"].as_str().is_some());
        assert!(result["message"]["text"].as_str().is_some());
        assert!(result["level"].as_str().is_some());
        
        let locations = result["locations"].as_array()
            .expect("Result should have locations array");
        assert!(!locations.is_empty(), "Result should have at least one location");
        
        let location = &locations[0];
        assert!(location["physicalLocation"]["artifactLocation"]["uri"].as_str().is_some());
        assert!(location["physicalLocation"]["region"]["startLine"].as_u64().is_some());
    }
}

#[test]
fn test_json_output_contains_violations() {
    let contract = r#"
        pub fn unsafe_math(x: i128, y: i128, z: i128) -> i128 {
            let a = x + y;
            let b = a - z;
            let c = b * 2;
            c / 10
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(contract);
    let (_exit_code, stdout, _stderr) = run_cli(&contract_path, "json");

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse JSON");

    let findings = json["findings"].as_array().unwrap();
    
    // Should detect +, -, *, / (4 operations, deduped per function)
    assert_eq!(findings.len(), 4);

    // Verify all operations are captured
    let operations: Vec<&str> = findings.iter()
        .filter_map(|f| f["message"].as_str())
        .collect();

    assert!(operations.iter().any(|m| m.contains("'+'")));
    assert!(operations.iter().any(|m| m.contains("'-'")));
    assert!(operations.iter().any(|m| m.contains("'*'")));
    assert!(operations.iter().any(|m| m.contains("'/'")));

    // Verify each finding has a suggestion
    for finding in findings {
        let suggestion = finding["suggestion"].as_str()
            .expect("Each finding should have a suggestion");
        assert!(!suggestion.is_empty());
        assert!(suggestion.contains("checked_") || suggestion.contains("saturating_"));
    }
}

#[test]
fn test_cli_exit_code_with_findings() {
    let contract_with_issues = r#"
        pub fn bad_math(a: u64) -> u64 {
            a + 1
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(contract_with_issues);
    let (exit_code, _stdout, _stderr) = run_cli(&contract_path, "json");

    // Should return non-zero when findings exist
    assert_ne!(exit_code, 0, "CLI should return non-zero with findings");
}

#[test]
fn test_multiple_findings_deduplication() {
    let contract = r#"
        pub fn dedupe_test(a: u64, b: u64, c: u64) -> u64 {
            let x = a + b;  // First +
            let y = a + c;  // Second + (should be deduped)
            let z = x + y;  // Third + (should be deduped)
            z
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(contract);
    let (_exit_code, stdout, _stderr) = run_cli(&contract_path, "json");

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse JSON");

    let findings = json["findings"].as_array().unwrap();
    
    // Should only report 1 finding due to per-function deduplication
    assert_eq!(findings.len(), 1, 
        "Multiple uses of same operator in one function should be deduplicated");
    
    let finding = &findings[0];
    assert!(finding["message"].as_str().unwrap().contains("'+' operation"));
    assert_eq!(finding["location"].as_str().unwrap(), "dedupe_test:3");
}

#[test]
fn test_safe_arithmetic_no_findings() {
    let safe_contract = r#"
        pub fn safe_math(a: u64, b: u64) -> Option<u64> {
            let sum = a.checked_add(b)?;
            let product = sum.checked_mul(2)?;
            Some(product)
        }

        pub fn saturating_math(x: i128, y: i128) -> i128 {
            let total = x.saturating_add(y);
            total.saturating_sub(100)
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(safe_contract);
    let (exit_code, stdout, _stderr) = run_cli(&contract_path, "json");

    // Should return zero when no findings
    assert_eq!(exit_code, 0, "CLI should return 0 when no findings");

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse JSON");

    let findings = json["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 0, "Safe arithmetic should produce no findings");
}

#[test]
fn test_skip_test_functions() {
    let contract = r#"
        pub fn production_code(amount: u64) -> u64 {
            amount + 100  // Should be flagged
        }

        #[test]
        fn test_helper() {
            let x = 1 + 2;  // Should NOT be flagged
            let y = x * 10; // Should NOT be flagged
            assert_eq!(y, 30);
        }

        #[cfg(test)]
        mod tests {
            fn another_test() {
                let a = 5 + 10;  // Should NOT be flagged
            }
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(contract);
    let (_exit_code, stdout, _stderr) = run_cli(&contract_path, "json");

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse JSON");

    let findings = json["findings"].as_array().unwrap();
    
    // Only production_code should trigger (1 finding)
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["location"].as_str().unwrap(), "production_code:2");
}

#[test]
fn test_custom_math_methods_detected() {
    let contract = r#"
        pub fn fixed_point_math(a: u64, b: u64) -> u64 {
            let result = a.mul_div(100, 50);  // Should be flagged
            result.fixed_point_mul(2)         // Should be flagged
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(contract);
    let (_exit_code, stdout, _stderr) = run_cli(&contract_path, "json");

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse JSON");

    let findings = json["findings"].as_array().unwrap();
    
    // Should detect both custom methods
    assert_eq!(findings.len(), 2);
    
    let messages: Vec<String> = findings.iter()
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect();
    
    assert!(messages.iter().any(|m| m.contains("mul_div")));
    assert!(messages.iter().any(|m| m.contains("fixed_point_mul")));
}

#[test]
fn test_index_subscript_arithmetic_not_flagged() {
    let contract = r#"
        pub fn read_buffer(buf: &[u8], index: usize) -> u8 {
            buf[index + 1]  // Should NOT be flagged (idiomatic pattern)
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(contract);
    let (exit_code, stdout, _stderr) = run_cli(&contract_path, "json");

    assert_eq!(exit_code, 0, "Index arithmetic should not be flagged");

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse JSON");

    let findings = json["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 0, "Index subscript arithmetic should not trigger findings");
}

#[test]
fn test_constant_folded_arithmetic_not_flagged() {
    let contract = r#"
        pub fn constants() -> u64 {
            let a = 1000 + 500;  // Compile-time constant, should NOT be flagged
            let b = 200 * 3;     // Compile-time constant, should NOT be flagged
            a + b
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(contract);
    let (exit_code, stdout, _stderr) = run_cli(&contract_path, "json");

    assert_eq!(exit_code, 0, "Constant-folded arithmetic should not be flagged");

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse JSON");

    let findings = json["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 0, "Literal arithmetic should not trigger findings");
}

#[test]
fn test_sarif_severity_mapping() {
    let contract = r#"
        pub fn test_severity(x: u64) -> u64 {
            x + 1
        }
    "#;

    let (_temp_dir, contract_path) = create_test_workspace(contract);
    let (_exit_code, stdout, _stderr) = run_cli(&contract_path, "sarif");

    let sarif: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse SARIF");

    let results = sarif["runs"][0]["results"].as_array().unwrap();
    assert!(!results.is_empty());

    let result = &results[0];
    let level = result["level"].as_str().unwrap();
    
    // S003 should map to "warning" in SARIF
    assert_eq!(level, "warning", "S003 should be mapped to 'warning' severity in SARIF");
}
