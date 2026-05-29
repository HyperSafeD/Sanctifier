//! Integration tests for security disclaimers
//!
//! These tests verify that security disclaimers work correctly in various scenarios.

use security_disclaimers::{DisclaimerCategory, SecurityDisclaimer, SecurityLevel};

#[test]
fn test_security_level_consistency() {
    let env = soroban_sdk::Env::default();

    // Test that all security levels work consistently across different categories
    for level in [
        SecurityLevel::Low,
        SecurityLevel::Medium,
        SecurityLevel::High,
        SecurityLevel::Critical,
    ] {
        for category in [
            DisclaimerCategory::Audit,
            DisclaimerCategory::Usage,
            DisclaimerCategory::Upgrade,
            DisclaimerCategory::Emergency,
        ] {
            let disclaimer = SecurityDisclaimer::get_disclaimer(env.clone(), level, category);

            // All disclaimers should be non-empty
            assert!(
                !disclaimer.is_empty(),
                "Disclaimer should not be empty for level {:?} and category {:?}",
                level,
                category
            );

            // All disclaimers should contain appropriate warnings
            match category {
                DisclaimerCategory::Audit => assert!(disclaimer.contains("SECURITY WARNING")),
                DisclaimerCategory::Usage => assert!(disclaimer.contains("PRODUCTION WARNING")),
                DisclaimerCategory::Upgrade => assert!(
                    disclaimer.contains("UPGRADE WARNING") || disclaimer.contains("UPGRADE INFO")
                ),
                DisclaimerCategory::Emergency => assert!(disclaimer.contains("EMERGENCY")),
            }
        }
    }
}

#[test]
fn test_multi_contract_security_levels() {
    let env = soroban_sdk::Env::default();

    // Test different contracts with different security levels
    let low_contract_disclaimer = SecurityDisclaimer::get_disclaimer(
        env.clone(),
        SecurityLevel::Low,
        DisclaimerCategory::Audit,
    );
    let critical_contract_disclaimer = SecurityDisclaimer::get_disclaimer(
        env.clone(),
        SecurityLevel::Critical,
        DisclaimerCategory::Audit,
    );

    // Critical contract should have stronger warnings
    assert!(critical_contract_disclaimer.len() > low_contract_disclaimer.len());
    assert!(critical_contract_disclaimer.contains("CRITICAL: Formal verification required"));
    assert!(!low_contract_disclaimer.contains("CRITICAL:"));
}

#[test]
fn test_disclaimer_content_validation() {
    let env = soroban_sdk::Env::default();

    // Test that disclaimer content is appropriate for each security level
    let critical_disclaimer = SecurityDisclaimer::get_disclaimer(
        env.clone(),
        SecurityLevel::Critical,
        DisclaimerCategory::Audit,
    );
    let high_disclaimer = SecurityDisclaimer::get_disclaimer(
        env.clone(),
        SecurityLevel::High,
        DisclaimerCategory::Audit,
    );
    let medium_disclaimer = SecurityDisclaimer::get_disclaimer(
        env.clone(),
        SecurityLevel::Medium,
        DisclaimerCategory::Audit,
    );
    let low_disclaimer = SecurityDisclaimer::get_disclaimer(
        env.clone(),
        SecurityLevel::Low,
        DisclaimerCategory::Audit,
    );

    // Critical should mention formal verification
    assert!(critical_disclaimer.contains("Formal verification"));

    // High should mention professional audit
    assert!(high_disclaimer.contains("Professional audit"));

    // Medium should mention security review
    assert!(medium_disclaimer.contains("Security review"));

    // Low should have basic warning
    assert!(low_disclaimer.contains("SECURITY WARNING"));
    assert!(!low_disclaimer.contains("Professional audit"));
    assert!(!low_disclaimer.contains("formal verification"));
}

#[test]
fn test_security_configuration_validation() {
    let env = soroban_sdk::Env::default();

    // Test valid security configurations
    assert!(SecurityDisclaimer::validate_security_config(
        env.clone(),
        SecurityLevel::Critical,
        true,
        true
    ));
    assert!(SecurityDisclaimer::validate_security_config(
        env.clone(),
        SecurityLevel::High,
        true,
        false
    ));
    assert!(SecurityDisclaimer::validate_security_config(
        env.clone(),
        SecurityLevel::Medium,
        false,
        false
    ));
    assert!(SecurityDisclaimer::validate_security_config(
        env.clone(),
        SecurityLevel::Low,
        false,
        false
    ));

    // Test invalid security configurations
    assert!(!SecurityDisclaimer::validate_security_config(
        env.clone(),
        SecurityLevel::Critical,
        true,
        false
    ));
    assert!(!SecurityDisclaimer::validate_security_config(
        env.clone(),
        SecurityLevel::Critical,
        false,
        true
    ));
    assert!(!SecurityDisclaimer::validate_security_config(
        env.clone(),
        SecurityLevel::Critical,
        false,
        false
    ));
    assert!(!SecurityDisclaimer::validate_security_config(
        env.clone(),
        SecurityLevel::High,
        false,
        true
    ));
    assert!(!SecurityDisclaimer::validate_security_config(
        env.clone(),
        SecurityLevel::High,
        false,
        false
    ));
}

#[test]
fn test_audit_requirements() {
    let env = soroban_sdk::Env::default();

    // Critical and High levels require audits
    assert!(SecurityDisclaimer::requires_audit(
        env.clone(),
        SecurityLevel::Critical
    ));
    assert!(SecurityDisclaimer::requires_audit(
        env.clone(),
        SecurityLevel::High
    ));

    // Medium and Low levels don't require audits
    assert!(!SecurityDisclaimer::requires_audit(
        env.clone(),
        SecurityLevel::Medium
    ));
    assert!(!SecurityDisclaimer::requires_audit(
        env.clone(),
        SecurityLevel::Low
    ));
}

#[test]
fn test_testing_requirements() {
    let env = soroban_sdk::Env::default();

    let critical_reqs =
        SecurityDisclaimer::get_testing_requirements(env.clone(), SecurityLevel::Critical);
    let high_reqs = SecurityDisclaimer::get_testing_requirements(env.clone(), SecurityLevel::High);
    let medium_reqs =
        SecurityDisclaimer::get_testing_requirements(env.clone(), SecurityLevel::Medium);
    let low_reqs = SecurityDisclaimer::get_testing_requirements(env.clone(), SecurityLevel::Low);

    // Critical should require formal verification
    assert!(critical_reqs.contains("Formal verification"));
    assert!(critical_reqs.contains("comprehensive audit"));

    // High should require professional audit
    assert!(high_reqs.contains("Professional audit"));
    assert!(high_reqs.contains("integration testing"));

    // Medium should require security review
    assert!(medium_reqs.contains("Security review"));
    assert!(medium_reqs.contains("unit testing"));

    // Low should require basic testing
    assert!(low_reqs.contains("Unit testing"));
    assert!(low_reqs.contains("basic security review"));
}

#[test]
fn test_contract_disclaimer_formatting() {
    use security_disclaimers::format_contract_disclaimer;

    let disclaimer = format_contract_disclaimer(SecurityLevel::High, "TestContract");

    // Should contain contract name
    assert!(disclaimer.contains("TestContract"));

    // Should contain security level
    assert!(disclaimer.contains("High"));

    // Should contain audit requirement
    assert!(disclaimer.contains("true"));

    // Should contain testing requirements
    assert!(disclaimer.contains("Professional audit"));

    // Should contain security warning
    assert!(disclaimer.contains("SECURITY WARNING"));
}
