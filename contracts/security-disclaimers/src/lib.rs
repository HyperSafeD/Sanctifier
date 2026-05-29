//! # Security Disclaimers Module
//!
//! This module provides standardized security disclaimers and safe usage guidelines
//! for Soroban smart contracts. It ensures consistent security messaging across
//! all contract implementations and provides runtime safety checks.
//!
//! ## Usage
//!
//! Add this to your contract's Cargo.toml:
//! ```toml
//! [dependencies]
//! security-disclaimers = { path = "../security-disclaimers" }
//! ```
//!
//! Then include in your contract:
//! ```rust
//! use security_disclaimers::{security_disclaimer, SecurityLevel};
//! ```
//!
//! ## Security Levels
//!
//! - **CRITICAL**: Contracts handling significant value or with complex governance
//! - **HIGH**: Contracts with user funds or sensitive operations
//! - **MEDIUM**: Contracts with limited risk exposure
//! - **LOW**: Utility contracts with minimal risk
//!
//! ## Disclaimer Categories
//!
//! - **AUDIT_STATUS**: Audit completion and recommendations
//! - **USAGE_RISKS**: Known risks and mitigation strategies
//! - **UPGRADE_PATHS**: Safe upgrade procedures
//! - **EMERGENCY_RESPONSE**: Crisis management procedures

#![no_std]

extern crate alloc;

use soroban_sdk::{contracttype, Env};

/// Security classification levels for contracts
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u8)]
pub enum SecurityLevel {
    /// Minimal risk, utility contracts
    Low = 0,
    /// Limited risk exposure
    Medium = 1,
    /// User funds or sensitive operations
    High = 2,
    /// Critical infrastructure or high-value contracts
    Critical = 3,
}

/// Security disclaimer categories
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DisclaimerCategory {
    Audit = 0,
    Usage = 1,
    Upgrade = 2,
    Emergency = 3,
}

/// Standard security disclaimer messages
pub const DISCLAIMER_AUDIT_REQUIRED: &str =
    "⚠️  SECURITY WARNING: This contract has not been audited. Use at your own risk. \
     Deploy only after thorough testing and security review.";

pub const DISCLAIMER_PRODUCTION_USE: &str =
    "⚠️  PRODUCTION WARNING: This contract is intended for testing purposes only. \
     Do not use in production without security audit and formal verification.";

pub const DISCLAIMER_UPGRADE_RISK: &str =
    "⚠️  UPGRADE WARNING: Contract upgrades may introduce security vulnerabilities. \
     Always verify upgrade logic and test thoroughly before deployment.";

pub const DISCLAIMER_ACCESS_CONTROL: &str =
    "⚠️  ACCESS CONTROL WARNING: Improper configuration of access controls may lead to \
     unauthorized access or fund loss. Review permissions carefully.";

pub const DISCLAIMER_TIME_SENSITIVE: &str =
    "⚠️  TIME-SENSITIVE WARNING: This contract depends on timing assumptions. \
     Network delays or clock variations may affect behavior.";

pub const DISCLAIMER_ORACLE_DEPENDENCY: &str =
    "⚠️  ORACLE WARNING: This contract depends on external price feeds. \
     Oracle manipulation or delays may impact contract behavior.";

pub const DISCLAIMER_COMPLEX_LOGIC: &str =
    "⚠️  COMPLEXITY WARNING: This contract contains complex logic that may have \
     edge cases. Comprehensive testing and formal verification recommended.";

/// Get the security disclaimer for a contract
///
/// # Arguments
/// * `env` - Soroban environment
/// * `level` - Security level of the contract
/// * `category` - Type of disclaimer needed
///
/// # Returns
/// String containing the appropriate disclaimer
pub fn get_disclaimer(
    env: Env,
    level: SecurityLevel,
    category: DisclaimerCategory,
) -> soroban_sdk::String {
    match (level, category) {
        (SecurityLevel::Critical, DisclaimerCategory::Audit) => {
            soroban_sdk::String::from_str(&env, "⚠️  SECURITY WARNING: This contract has not been audited. Use at your own risk. Deploy only after thorough testing and security review. CRITICAL: Formal verification required.")
        }
        (SecurityLevel::High, DisclaimerCategory::Audit) => {
            soroban_sdk::String::from_str(&env, "⚠️  SECURITY WARNING: This contract has not been audited. Use at your own risk. Deploy only after thorough testing and security review. HIGH: Professional audit strongly recommended.")
        }
        (SecurityLevel::Medium, DisclaimerCategory::Audit) => {
            soroban_sdk::String::from_str(&env, "⚠️  SECURITY WARNING: This contract has not been audited. Use at your own risk. Deploy only after thorough testing and security review. MEDIUM: Security review recommended.")
        }
        (SecurityLevel::Low, DisclaimerCategory::Audit) => {
            soroban_sdk::String::from_str(&env, "⚠️  SECURITY WARNING: This contract has not been audited. Use at your own risk. Deploy only after thorough testing and security review.")
        }
        (SecurityLevel::Critical, DisclaimerCategory::Usage) => {
            soroban_sdk::String::from_str(&env, "⚠️  PRODUCTION WARNING: This contract is intended for testing purposes only. Do not use in production without security audit and formal verification. CRITICAL: Extensive testing required.")
        }
        (SecurityLevel::High, DisclaimerCategory::Usage) => {
            soroban_sdk::String::from_str(&env, "⚠️  PRODUCTION WARNING: This contract is intended for testing purposes only. Do not use in production without security audit and formal verification. HIGH: Comprehensive testing required.")
        }
        (SecurityLevel::Medium, DisclaimerCategory::Usage) => {
            soroban_sdk::String::from_str(&env, "⚠️  PRODUCTION WARNING: This contract is intended for testing purposes only. Do not use in production without security audit and formal verification. MEDIUM: Thorough testing recommended.")
        }
        (SecurityLevel::Low, DisclaimerCategory::Usage) => {
            soroban_sdk::String::from_str(&env, "⚠️  PRODUCTION WARNING: This contract is intended for testing purposes only. Do not use in production without security audit and formal verification.")
        }
        (SecurityLevel::Critical, DisclaimerCategory::Upgrade) => {
            soroban_sdk::String::from_str(&env, "⚠️  UPGRADE WARNING: Contract upgrades may introduce security vulnerabilities. Always verify upgrade logic and test thoroughly before deployment. CRITICAL: Upgrade requires governance approval.")
        }
        (SecurityLevel::High, DisclaimerCategory::Upgrade) => {
            soroban_sdk::String::from_str(&env, "⚠️  UPGRADE WARNING: Contract upgrades may introduce security vulnerabilities. Always verify upgrade logic and test thoroughly before deployment. HIGH: Upgrade requires multi-signature approval.")
        }
        (SecurityLevel::Medium, DisclaimerCategory::Upgrade) => {
            soroban_sdk::String::from_str(&env, "⚠️  UPGRADE WARNING: Contract upgrades may introduce security vulnerabilities. Always verify upgrade logic and test thoroughly before deployment. MEDIUM: Upgrade requires admin approval.")
        }
        (SecurityLevel::Low, DisclaimerCategory::Upgrade) => {
            soroban_sdk::String::from_str(&env, "⚠️  UPGRADE WARNING: Contract upgrades may introduce security vulnerabilities. Always verify upgrade logic and test thoroughly before deployment. This contract supports upgrades.")
        }
        (SecurityLevel::Critical, DisclaimerCategory::Emergency) => {
            soroban_sdk::String::from_str(&env, "EMERGENCY: Critical security issue detected. Contact development team immediately.")
        }
        (SecurityLevel::High, DisclaimerCategory::Emergency) => {
            soroban_sdk::String::from_str(&env, "EMERGENCY: High priority security issue. Contact development team immediately.")
        }
        (SecurityLevel::Medium, DisclaimerCategory::Emergency) => {
            soroban_sdk::String::from_str(&env, "EMERGENCY: Medium priority security issue. Contact development team immediately.")
        }
        (SecurityLevel::Low, DisclaimerCategory::Emergency) => {
            soroban_sdk::String::from_str(&env, "EMERGENCY: Low priority security issue. Contact development team immediately.")
        }
    }
}

/// Check if contract requires audit based on security level
pub fn requires_audit(_env: Env, level: SecurityLevel) -> bool {
    matches!(level, SecurityLevel::High | SecurityLevel::Critical)
}

/// Get recommended testing requirements
pub fn get_testing_requirements(env: Env, level: SecurityLevel) -> soroban_sdk::String {
    match level {
        SecurityLevel::Critical => {
            soroban_sdk::String::from_str(&env, "Requirements: Formal verification, comprehensive audit, stress testing, security review")
        }
        SecurityLevel::High => {
            soroban_sdk::String::from_str(&env, "Requirements: Professional audit, integration testing, security review")
        }
        SecurityLevel::Medium => {
            soroban_sdk::String::from_str(&env, "Requirements: Security review, unit testing, integration testing")
        }
        SecurityLevel::Low => {
            soroban_sdk::String::from_str(&env, "Requirements: Unit testing, basic security review")
        }
    }
}

/// Validate security configuration
pub fn validate_security_config(
    _env: Env,
    level: SecurityLevel,
    has_admin: bool,
    has_upgrade: bool,
) -> bool {
    match level {
        SecurityLevel::Critical => has_admin && has_upgrade,
        SecurityLevel::High => has_admin,
        SecurityLevel::Medium => true,
        SecurityLevel::Low => true,
    }
}

/// Helper macro for adding security disclaimers to contracts
#[macro_export]
macro_rules! security_disclaimer {
    ($level:expr) => {
        concat!(
            "\n\n=== SECURITY DISCLAIMER ===\n",
            "This contract is classified as ",
            stringify!($level),
            " security level.\n",
            "Use only after appropriate security review and testing.\n",
            "See documentation for detailed security guidelines.\n",
            "=============================\n"
        )
    };
}

/// Helper function to format security disclaimer for contract documentation
#[cfg(not(target_arch = "wasm32"))]
pub fn format_contract_disclaimer(
    level: SecurityLevel,
    contract_name: &str,
) -> alloc::string::String {
    let mut result = alloc::string::String::from("\n\n## 🔐 Security Disclaimer\n\n");
    result += "**Contract:** ";
    result += contract_name;
    result += "\n**Security Level:** ";

    // Convert security level to string representation
    let level_str = match level {
        SecurityLevel::Low => "Low",
        SecurityLevel::Medium => "Medium",
        SecurityLevel::High => "High",
        SecurityLevel::Critical => "Critical",
    };

    result += level_str;
    result += "\n**Audit Required:** ";

    let audit_required = requires_audit(Env::default(), level);
    result += if audit_required { "true" } else { "false" };

    result += "\n\n";
    result += "**Security Warning:** This contract has not been audited. Use at your own risk.\n";
    result += "**Testing Requirements:** ";

    let testing_req = match level {
        SecurityLevel::Critical => {
            "Formal verification, comprehensive audit, stress testing, security review"
        }
        SecurityLevel::High => "Professional audit, integration testing, security review",
        SecurityLevel::Medium => "Security review, unit testing, integration testing",
        SecurityLevel::Low => "Unit testing, basic security review",
    };

    result += testing_req;
    result += "\n\nUse this contract only after understanding the risks and implementing appropriate security measures.\n";

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_levels() {
        assert!(SecurityLevel::Critical > SecurityLevel::High);
        assert!(SecurityLevel::High > SecurityLevel::Medium);
        assert!(SecurityLevel::Medium > SecurityLevel::Low);
    }

    #[test]
    fn test_audit_requirements() {
        let env = Env::default();
        assert!(SecurityDisclaimer::requires_audit(
            env.clone(),
            SecurityLevel::Critical
        ));
        assert!(SecurityDisclaimer::requires_audit(
            env.clone(),
            SecurityLevel::High
        ));
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
    fn test_security_config_validation() {
        let env = Env::default();

        // Critical level requires both admin and upgrade
        assert!(SecurityDisclaimer::validate_security_config(
            env.clone(),
            SecurityLevel::Critical,
            true,
            true
        ));
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

        // High level requires admin
        assert!(SecurityDisclaimer::validate_security_config(
            env.clone(),
            SecurityLevel::High,
            true,
            false
        ));
        assert!(SecurityDisclaimer::validate_security_config(
            env.clone(),
            SecurityLevel::High,
            true,
            true
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

        // Medium and Low levels have no requirements
        assert!(SecurityDisclaimer::validate_security_config(
            env.clone(),
            SecurityLevel::Medium,
            false,
            false
        ));
        assert!(SecurityDisclaimer::validate_security_config(
            env.clone(),
            SecurityLevel::Medium,
            true,
            true
        ));
        assert!(SecurityDisclaimer::validate_security_config(
            env.clone(),
            SecurityLevel::Low,
            false,
            false
        ));
        assert!(SecurityDisclaimer::validate_security_config(
            env.clone(),
            SecurityLevel::Low,
            true,
            true
        ));
    }

    #[test]
    fn test_disclaimer_formatting() {
        let disclaimer = format_contract_disclaimer(SecurityLevel::High, "TestContract");
        assert!(disclaimer.contains("TestContract"));
        assert!(disclaimer.contains("High"));
        assert!(disclaimer.contains("true"));
    }
}
