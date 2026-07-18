//! Integration tests for SEP-41 (S012) interface compliance checking.
//!
//! These tests verify the robustness of the SEP-41 checker across various
//! contract scenarios including edge cases, partial implementations, and
//! common mistakes.

use sanctifier_core::sep41::{self, Sep41IssueKind};

// ============================================================================
// Full Compliance Tests
// ============================================================================

#[test]
fn test_fully_compliant_sep41_token() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl Token {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
                from.require_auth();
            }
            
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                from.require_auth();
            }
            
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                spender.require_auth();
            }
            
            pub fn burn(env: Env, from: Address, amount: i128) {
                from.require_auth();
            }
            
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                spender.require_auth();
            }
            
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(report.candidate, "Should be recognized as token candidate");
    assert!(report.compliant, "Should be fully compliant");
    assert_eq!(report.issues.len(), 0, "Should have zero issues");
    assert_eq!(
        report.verified_functions.len(),
        10,
        "Should verify all 10 functions"
    );

    // Verify all functions are in the list
    let expected_functions = [
        "allowance",
        "approve",
        "balance",
        "burn",
        "burn_from",
        "decimals",
        "name",
        "symbol",
        "transfer",
        "transfer_from",
    ];
    for func in expected_functions {
        assert!(
            report.verified_functions.contains(&func.to_string()),
            "Missing verified function: {}",
            func
        );
    }
}

// ============================================================================
// Missing Function Tests
// ============================================================================

#[test]
fn test_missing_multiple_functions() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl IncompleteToken {
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                from.require_auth();
            }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(report.candidate, "Should be recognized as token candidate");
    assert!(!report.compliant, "Should not be compliant");

    let missing_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.kind == Sep41IssueKind::MissingFunction)
        .collect();

    assert!(
        missing_issues.len() >= 5,
        "Should report multiple missing functions"
    );

    // Verify specific missing functions
    let missing_names: Vec<&str> = missing_issues
        .iter()
        .map(|i| i.function_name.as_str())
        .collect();

    assert!(
        missing_names.contains(&"allowance"),
        "Should report missing allowance"
    );
    assert!(
        missing_names.contains(&"approve"),
        "Should report missing approve"
    );
    assert!(
        missing_names.contains(&"transfer_from"),
        "Should report missing transfer_from"
    );
    assert!(
        missing_names.contains(&"burn"),
        "Should report missing burn"
    );
    assert!(
        missing_names.contains(&"burn_from"),
        "Should report missing burn_from"
    );
}

// ============================================================================
// Signature Mismatch Tests
// ============================================================================

#[test]
fn test_wrong_parameter_types() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl Token {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
                from.require_auth();
            }
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            
            // Wrong: should use MuxedAddress for 'to' parameter
            pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                from.require_auth();
            }
            
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn burn(env: Env, from: Address, amount: i128) {
                from.require_auth();
            }
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(report.candidate);
    assert!(!report.compliant);

    let sig_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.kind == Sep41IssueKind::SignatureMismatch && i.function_name == "transfer")
        .collect();

    assert_eq!(
        sig_issues.len(),
        1,
        "Should report transfer signature mismatch"
    );
    assert!(sig_issues[0]
        .message
        .contains("does not match the exact SEP-41 signature"));
    assert!(
        sig_issues[0].actual_signature.is_some(),
        "Should include actual signature"
    );
}

#[test]
fn test_wrong_return_type() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl Token {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
                from.require_auth();
            }
            
            // Wrong: should return i128, not u64
            pub fn balance(env: Env, id: Address) -> u64 { 0 }
            
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                from.require_auth();
            }
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn burn(env: Env, from: Address, amount: i128) {
                from.require_auth();
            }
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(!report.compliant);

    let balance_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.function_name == "balance" && i.kind == Sep41IssueKind::SignatureMismatch)
        .collect();

    assert_eq!(
        balance_issues.len(),
        1,
        "Should report balance return type mismatch"
    );
}

#[test]
fn test_missing_parameter() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl Token {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            
            // Wrong: missing expiration_ledger parameter
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128) {
                from.require_auth();
            }
            
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                from.require_auth();
            }
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn burn(env: Env, from: Address, amount: i128) {
                from.require_auth();
            }
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(!report.compliant);

    let approve_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.function_name == "approve" && i.kind == Sep41IssueKind::SignatureMismatch)
        .collect();

    assert_eq!(
        approve_issues.len(),
        1,
        "Should report approve signature mismatch"
    );
}

// ============================================================================
// Authorization Mismatch Tests
// ============================================================================

#[test]
fn test_missing_authorization() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl Token {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            
            // Missing: from.require_auth()
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
                // No authorization!
            }
            
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                from.require_auth();
            }
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn burn(env: Env, from: Address, amount: i128) {
                from.require_auth();
            }
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(!report.compliant);

    let auth_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.kind == Sep41IssueKind::AuthorizationMismatch && i.function_name == "approve")
        .collect();

    assert_eq!(
        auth_issues.len(),
        1,
        "Should report missing authorization in approve"
    );
    assert!(auth_issues[0].message.contains("should authorize 'from'"));
}

#[test]
fn test_wrong_parameter_authorized() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl Token {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
                from.require_auth();
            }
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                from.require_auth();
            }
            
            // Wrong: should authorize spender, not from
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                from.require_auth();
            }
            
            pub fn burn(env: Env, from: Address, amount: i128) {
                from.require_auth();
            }
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(!report.compliant);

    let transfer_from_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| {
            i.kind == Sep41IssueKind::AuthorizationMismatch && i.function_name == "transfer_from"
        })
        .collect();

    assert_eq!(
        transfer_from_issues.len(),
        1,
        "Should report wrong authorization in transfer_from"
    );
    assert!(transfer_from_issues[0]
        .message
        .contains("should authorize 'spender'"));
}

#[test]
fn test_multiple_authorization_issues() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl Token {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            
            // Missing auth
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {}
            
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            
            // Missing auth
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {}
            
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                spender.require_auth();
            }
            
            // Missing auth
            pub fn burn(env: Env, from: Address, amount: i128) {}
            
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(!report.compliant);

    let auth_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.kind == Sep41IssueKind::AuthorizationMismatch)
        .collect();

    assert_eq!(
        auth_issues.len(),
        3,
        "Should report all 3 missing authorizations"
    );

    let function_names: Vec<&str> = auth_issues
        .iter()
        .map(|i| i.function_name.as_str())
        .collect();

    assert!(function_names.contains(&"approve"));
    assert!(function_names.contains(&"transfer"));
    assert!(function_names.contains(&"burn"));
}

// ============================================================================
// Mixed Issue Tests
// ============================================================================

#[test]
fn test_all_three_issue_types_together() {
    let source = r#"
        use soroban_sdk::{Address, Env, String};

        #[contractimpl]
        impl Token {
            // Missing: allowance (MissingFunction)
            
            // Present but missing auth (AuthorizationMismatch)
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {}
            
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            
            // Wrong signature - Address instead of MuxedAddress (SignatureMismatch)
            pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                from.require_auth();
            }
            
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                spender.require_auth();
            }
            
            // Missing: burn (MissingFunction)
            // Missing: burn_from (MissingFunction)
            
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(report.candidate);
    assert!(!report.compliant);

    // Count each issue type
    let missing_count = report
        .issues
        .iter()
        .filter(|i| i.kind == Sep41IssueKind::MissingFunction)
        .count();
    let signature_count = report
        .issues
        .iter()
        .filter(|i| i.kind == Sep41IssueKind::SignatureMismatch)
        .count();
    let auth_count = report
        .issues
        .iter()
        .filter(|i| i.kind == Sep41IssueKind::AuthorizationMismatch)
        .count();

    assert!(
        missing_count >= 3,
        "Should have at least 3 missing functions (allowance, burn, burn_from)"
    );
    assert_eq!(
        signature_count, 1,
        "Should have 1 signature mismatch (transfer)"
    );
    assert_eq!(
        auth_count, 1,
        "Should have 1 authorization mismatch (approve)"
    );
}

// ============================================================================
// Candidate Detection Tests
// ============================================================================

#[test]
fn test_non_token_contract_not_candidate() {
    let source = r#"
        use soroban_sdk::Env;

        #[contractimpl]
        impl Counter {
            pub fn increment(env: Env) {}
            pub fn get(env: Env) -> u32 { 0 }
            pub fn reset(env: Env) {}
        }
    "#;

    let report = sep41::verify(source);

    assert!(
        !report.candidate,
        "Non-token contract should not be candidate"
    );
    assert!(!report.compliant);
    assert_eq!(
        report.issues.len(),
        0,
        "Non-candidates should have no issues"
    );
}

#[test]
fn test_minimal_token_candidate_two_core_functions() {
    let source = r#"
        use soroban_sdk::{Address, Env};

        #[contractimpl]
        impl Token {
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {}
        }
    "#;

    let report = sep41::verify(source);

    assert!(
        report.candidate,
        "Should be candidate with 2 core functions"
    );
    assert!(!report.compliant, "Should not be compliant");
    assert!(!report.issues.is_empty(), "Should report missing functions");
}

#[test]
fn test_minimal_token_candidate_one_core_two_metadata() {
    let source = r#"
        use soroban_sdk::{Address, Env, String};

        #[contractimpl]
        impl Token {
            pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {}
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(
        report.candidate,
        "Should be candidate with 1 core + 2 metadata"
    );
    assert!(!report.compliant);
}

#[test]
fn test_not_candidate_only_one_function() {
    let source = r#"
        use soroban_sdk::{Address, Env};

        #[contractimpl]
        impl Token {
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
        }
    "#;

    let report = sep41::verify(source);

    assert!(
        !report.candidate,
        "Single function should not make candidate"
    );
}

// ============================================================================
// Edge Cases and Robustness Tests
// ============================================================================

#[test]
fn test_parse_error_returns_non_candidate() {
    let source = r#"
        This is not valid Rust code { } } {
    "#;

    let report = sep41::verify(source);

    assert!(
        !report.candidate,
        "Parse errors should return non-candidate"
    );
    assert!(!report.compliant);
    assert_eq!(report.issues.len(), 0);
}

#[test]
fn test_empty_source() {
    let report = sep41::verify("");

    assert!(!report.candidate);
    assert!(!report.compliant);
    assert_eq!(report.issues.len(), 0);
}

#[test]
fn test_private_functions_ignored() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl Token {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
                from.require_auth();
            }
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                from.require_auth();
            }
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn burn(env: Env, from: Address, amount: i128) {
                from.require_auth();
            }
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
            
            // Private functions should be ignored
            fn internal_helper(env: Env) {}
            fn validate_amount(amount: i128) -> bool { true }
        }
    "#;

    let report = sep41::verify(source);

    assert!(
        report.compliant,
        "Private functions should not affect compliance"
    );
}

#[test]
fn test_deterministic_output_order() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl Token {
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                from.require_auth();
            }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    // Run verification multiple times
    let report1 = sep41::verify(source);
    let report2 = sep41::verify(source);
    let report3 = sep41::verify(source);

    // All runs should produce identical results
    assert_eq!(report1.candidate, report2.candidate);
    assert_eq!(report1.compliant, report2.compliant);
    assert_eq!(report1.verified_functions, report2.verified_functions);
    assert_eq!(report1.verified_functions, report3.verified_functions);
    assert_eq!(report1.issues.len(), report2.issues.len());
    assert_eq!(report1.issues.len(), report3.issues.len());
}

// ============================================================================
// Authorization Detection Tests
// ============================================================================

#[test]
fn test_require_auth_for_args_detected() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String, Vec};

        #[contractimpl]
        impl Token {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
                from.require_auth_for_args(Vec::new(&env));
            }
            
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                from.require_auth();
            }
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn burn(env: Env, from: Address, amount: i128) {
                from.require_auth();
            }
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    assert!(
        report.compliant,
        "require_auth_for_args should be recognized as valid authorization"
    );
}

#[test]
fn test_authorization_in_nested_scope() {
    let source = r#"
        use soroban_sdk::{Address, Env, MuxedAddress, String};

        #[contractimpl]
        impl Token {
            pub fn allowance(env: Env, from: Address, spender: Address) -> i128 { 0 }
            
            pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
                if amount > 0 {
                    from.require_auth();
                }
            }
            
            pub fn balance(env: Env, id: Address) -> i128 { 0 }
            pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
                from.require_auth();
            }
            pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn burn(env: Env, from: Address, amount: i128) {
                from.require_auth();
            }
            pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
                spender.require_auth();
            }
            pub fn decimals(env: Env) -> u32 { 7 }
            pub fn name(env: Env) -> String { String::from_str(&env, "Token") }
            pub fn symbol(env: Env) -> String { String::from_str(&env, "TOK") }
        }
    "#;

    let report = sep41::verify(source);

    // Current implementation detects require_auth in nested scopes
    assert!(report.compliant, "Nested authorization should be detected");
}
