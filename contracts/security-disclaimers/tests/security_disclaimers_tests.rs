use security_disclaimers::{
    get_disclaimer, get_testing_requirements, requires_audit, security_disclaimer,
    validate_security_config, DisclaimerCategory, SecurityLevel,
};
use soroban_sdk::Env;

#[test]
fn test_security_level_ordering() {
    // Test that security levels are properly ordered
    assert!(SecurityLevel::Critical > SecurityLevel::High);
    assert!(SecurityLevel::High > SecurityLevel::Medium);
    assert!(SecurityLevel::Medium > SecurityLevel::Low);

    // Test specific values
    assert_eq!(SecurityLevel::Low as u8, 0);
    assert_eq!(SecurityLevel::Medium as u8, 1);
    assert_eq!(SecurityLevel::High as u8, 2);
    assert_eq!(SecurityLevel::Critical as u8, 3);
}

#[test]
fn test_disclaimer_category_values() {
    assert_eq!(DisclaimerCategory::Audit as u8, 0);
    assert_eq!(DisclaimerCategory::Usage as u8, 1);
    assert_eq!(DisclaimerCategory::Upgrade as u8, 2);
    assert_eq!(DisclaimerCategory::Emergency as u8, 3);
}

#[test]
fn test_audit_requirements() {
    let env = Env::default();

    // Critical and High levels require audits
    assert!(requires_audit(env.clone(), SecurityLevel::Critical));
    assert!(requires_audit(env.clone(), SecurityLevel::High));

    // Medium and Low levels don't require audits
    assert!(!requires_audit(env.clone(), SecurityLevel::Medium));
    assert!(!requires_audit(env.clone(), SecurityLevel::Low));
}

#[test]
fn test_security_config_validation() {
    let env = Env::default();

    // Critical level requires both admin and upgrade
    assert!(validate_security_config(
        env.clone(),
        SecurityLevel::Critical,
        true,
        true
    ));
    assert!(!validate_security_config(
        env.clone(),
        SecurityLevel::Critical,
        true,
        false
    ));
    assert!(!validate_security_config(
        env.clone(),
        SecurityLevel::Critical,
        false,
        true
    ));
    assert!(!validate_security_config(
        env.clone(),
        SecurityLevel::Critical,
        false,
        false
    ));

    // High level requires admin
    assert!(validate_security_config(
        env.clone(),
        SecurityLevel::High,
        true,
        false
    ));
    assert!(validate_security_config(
        env.clone(),
        SecurityLevel::High,
        true,
        true
    ));
    assert!(!validate_security_config(
        env.clone(),
        SecurityLevel::High,
        false,
        true
    ));
    assert!(!validate_security_config(
        env.clone(),
        SecurityLevel::High,
        false,
        false
    ));

    // Medium and Low levels have no requirements
    assert!(validate_security_config(
        env.clone(),
        SecurityLevel::Medium,
        false,
        false
    ));
    assert!(validate_security_config(
        env.clone(),
        SecurityLevel::Medium,
        true,
        true
    ));
    assert!(validate_security_config(
        env.clone(),
        SecurityLevel::Low,
        false,
        false
    ));
    assert!(validate_security_config(
        env.clone(),
        SecurityLevel::Low,
        true,
        true
    ));
}

#[test]
fn test_audit_disclaimers() {
    let env = Env::default();

    let critical_audit = get_disclaimer(
        env.clone(),
        SecurityLevel::Critical,
        DisclaimerCategory::Audit,
    )
    .to_string();
    let high_audit =
        get_disclaimer(env.clone(), SecurityLevel::High, DisclaimerCategory::Audit).to_string();
    let medium_audit = get_disclaimer(
        env.clone(),
        SecurityLevel::Medium,
        DisclaimerCategory::Audit,
    )
    .to_string();
    let low_audit =
        get_disclaimer(env.clone(), SecurityLevel::Low, DisclaimerCategory::Audit).to_string();

    // All should contain the basic audit warning
    assert!(critical_audit.contains("SECURITY WARNING"));
    assert!(high_audit.contains("SECURITY WARNING"));
    assert!(medium_audit.contains("SECURITY WARNING"));
    assert!(low_audit.contains("SECURITY WARNING"));

    // Critical should mention formal verification
    assert!(critical_audit.contains("CRITICAL: Formal verification required"));

    // High should mention professional audit
    assert!(high_audit.contains("HIGH: Professional audit strongly recommended"));

    // Medium should mention security review
    assert!(medium_audit.contains("MEDIUM: Security review recommended"));

    // Low should not have additional qualifiers
    assert!(!low_audit.contains("CRITICAL:"));
    assert!(!low_audit.contains("HIGH:"));
    assert!(!low_audit.contains("MEDIUM:"));
}

#[test]
fn test_usage_disclaimers() {
    let env = Env::default();

    let critical_usage = get_disclaimer(
        env.clone(),
        SecurityLevel::Critical,
        DisclaimerCategory::Usage,
    )
    .to_string();
    let high_usage =
        get_disclaimer(env.clone(), SecurityLevel::High, DisclaimerCategory::Usage).to_string();

    // All should contain the production warning
    assert!(critical_usage.contains("PRODUCTION WARNING"));
    assert!(high_usage.contains("PRODUCTION WARNING"));

    // Critical should mention extensive testing
    assert!(critical_usage.contains("CRITICAL: Extensive testing required"));

    // High should mention comprehensive testing
    assert!(high_usage.contains("HIGH: Comprehensive testing required"));
}

#[test]
fn test_upgrade_disclaimers() {
    let env = Env::default();

    let critical_upgrade = get_disclaimer(
        env.clone(),
        SecurityLevel::Critical,
        DisclaimerCategory::Upgrade,
    )
    .to_string();
    let high_upgrade = get_disclaimer(
        env.clone(),
        SecurityLevel::High,
        DisclaimerCategory::Upgrade,
    )
    .to_string();
    let medium_upgrade = get_disclaimer(
        env.clone(),
        SecurityLevel::Medium,
        DisclaimerCategory::Upgrade,
    )
    .to_string();
    let low_upgrade =
        get_disclaimer(env.clone(), SecurityLevel::Low, DisclaimerCategory::Upgrade).to_string();

    // All should contain the upgrade warning
    assert!(critical_upgrade.contains("UPGRADE WARNING"));
    assert!(high_upgrade.contains("UPGRADE WARNING"));
    assert!(medium_upgrade.contains("UPGRADE WARNING"));
    assert!(low_upgrade.contains("UPGRADE INFO"));

    // Critical should mention governance approval
    assert!(critical_upgrade.contains("CRITICAL: Upgrade requires governance approval"));

    // High should mention multi-signature approval
    assert!(high_upgrade.contains("HIGH: Upgrade requires multi-signature approval"));

    // Low should have informational message
    assert!(low_upgrade.contains("This contract supports upgrades"));
}

#[test]
fn test_emergency_disclaimers() {
    let env = Env::default();

    let emergency = get_disclaimer(
        env.clone(),
        SecurityLevel::Critical,
        DisclaimerCategory::Emergency,
    )
    .to_string();

    // All emergency disclaimers should be the same regardless of level
    assert!(emergency.contains("EMERGENCY"));
    assert!(emergency.contains("contact development team immediately"));

    let emergency_medium = get_disclaimer(
        env.clone(),
        SecurityLevel::Medium,
        DisclaimerCategory::Emergency,
    )
    .to_string();
    assert_eq!(emergency, emergency_medium);
}

#[test]
fn test_testing_requirements() {
    let env = Env::default();

    let critical_reqs = get_testing_requirements(env.clone(), SecurityLevel::Critical).to_string();
    let high_reqs = get_testing_requirements(env.clone(), SecurityLevel::High).to_string();
    let medium_reqs = get_testing_requirements(env.clone(), SecurityLevel::Medium).to_string();
    let low_reqs = get_testing_requirements(env.clone(), SecurityLevel::Low).to_string();

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
fn test_edge_cases() {
    let env = Env::default();

    // Test all combinations of security levels and categories
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
            let env_clone = env.clone();
            let disclaimer = get_disclaimer(env_clone, level, category).to_string();

            // All disclaimers should be non-empty
            assert!(!disclaimer.is_empty());

            // All disclaimers should contain appropriate content
            assert!(!disclaimer.is_empty());
            assert!(disclaimer.len() > 10); // Basic sanity check
        }
    }
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
    assert!(disclaimer.contains("Security Warning"));
}

#[test]
fn test_macro_expansion() {
    // Test that the security_disclaimer macro compiles and produces expected output
    let disclaimer = security_disclaimer!(SecurityLevel::Critical);

    assert!(disclaimer.contains("SECURITY DISCLAIMER"));
    assert!(disclaimer.contains("Critical"));
    assert!(disclaimer.contains("security level"));
}
