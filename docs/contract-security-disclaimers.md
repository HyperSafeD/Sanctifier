# Contract Security Disclaimers Guide

This document provides comprehensive guidance on implementing and using security disclaimers in Soroban smart contracts within the Sanctifier ecosystem.

## Overview

The security disclaimer system provides standardized security messaging and safe usage guidelines for smart contracts. It ensures consistent security communication across all contract implementations and provides runtime safety checks.

## 🔐 Security Levels

### Critical (Level 3)
**Use Cases:** Contracts handling significant value, complex governance, or critical infrastructure
- **Audit Required:** Yes - Formal verification required
- **Testing Requirements:** Formal verification, comprehensive audit, stress testing, security review
- **Examples:** Multisig wallets, governance contracts, proxy contracts with upgrade capabilities

### High (Level 2)
**Use Cases:** Contracts with user funds or sensitive operations
- **Audit Required:** Yes - Professional audit strongly recommended  
- **Testing Requirements:** Professional audit, integration testing, security review
- **Examples:** AMM pools, token contracts with minting, flashloan contracts

### Medium (Level 1)
**Use Cases:** Contracts with limited risk exposure
- **Audit Required:** No - Security review recommended
- **Testing Requirements:** Security review, unit testing, integration testing
- **Examples:** Vesting contracts, timelock contracts, oracle interfaces

### Low (Level 0)
**Use Cases:** Utility contracts with minimal risk
- **Audit Required:** No
- **Testing Requirements:** Unit testing, basic security review
- **Examples:** Simple utility contracts, interface contracts

## 📋 Disclaimer Categories

### Audit Disclaimers
Provide information about audit status and requirements for the contract.

### Usage Disclaimers
Warn about production usage risks and testing requirements.

### Upgrade Disclaimers
Inform about upgrade risks and procedures for upgradeable contracts.

### Emergency Disclaimers
Provide emergency contact and response procedures.

## 🛠️ Implementation Guide

### 1. Add Dependency

Add to your contract's `Cargo.toml`:
```toml
[dependencies]
security-disclaimers = { path = "../security-disclaimers" }
```

### 2. Import and Use

```rust
use security_disclaimers::{SecurityLevel, DisclaimerCategory, SecurityDisclaimer};

#[contractimpl]
impl YourContract {
    /// Get security disclaimer for this contract
    pub fn get_security_disclaimer(env: Env, category: DisclaimerCategory) -> String {
        SecurityDisclaimer::get_disclaimer(env, SecurityLevel::High, category)
    }

    /// Validate security configuration
    pub fn validate_security_config(env: Env, has_admin: bool, has_upgrade: bool) -> bool {
        SecurityDisclaimer::validate_security_config(env, SecurityLevel::High, has_admin, has_upgrade)
    }
}
```

### 3. Add Documentation

Include security disclaimers in your contract documentation:

```rust
//! ## 🔐 Security Disclaimer
//!
//! **Contract:** Your Contract Name  
//! **Security Level:** High  
//! **Audit Required:** true  
//!
//! ⚠️  SECURITY WARNING: This contract has not been audited. Use at your own risk. Deploy only after thorough testing and security review. HIGH: Professional audit strongly recommended.
//!
//! **Testing Requirements:** Requirements: Professional audit, integration testing, security review
//!
//! Use this contract only after understanding the risks and implementing appropriate security measures.
```

## 📖 Usage Examples

### Basic Usage

```rust
use security_disclaimers::{SecurityLevel, DisclaimerCategory, SecurityDisclaimer};

// Get audit disclaimer
let audit_warning = SecurityDisclaimer::get_disclaimer(&env, SecurityLevel::High, DisclaimerCategory::Audit);

// Check if audit is required
if SecurityDisclaimer::requires_audit(&env, SecurityLevel::High) {
    // Require audit before deployment
}

// Validate security configuration
let is_secure = SecurityDisclaimer::validate_security_config(&env, SecurityLevel::High, true, false);
```

### Contract Integration

```rust
#[contractimpl]
impl SecureToken {
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        // Validate security configuration before sensitive operations
        let is_secure = Self::validate_security_config(&env, true, false);
        if !is_secure {
            env.panic_with_error(Error::SecurityCheckFailed);
        }
        
        // Continue with transfer logic
        // ...
    }
    
    pub fn get_security_info(env: Env) -> (String, bool, String) {
        let audit_disclaimer = Self::get_security_disclaimer(&env, DisclaimerCategory::Audit);
        let requires_audit = SecurityDisclaimer::requires_audit(&env, SecurityLevel::High);
        let testing_reqs = SecurityDisclaimer::get_testing_requirements(&env, SecurityLevel::High);
        
        (audit_disclaimer, requires_audit, testing_reqs)
    }
}
```

## 🧪 Testing

### Unit Tests

The security disclaimer module includes comprehensive unit tests. Run them with:

```bash
cargo test -p security-disclaimers
```

### Integration Tests

Integration tests verify security disclaimer functionality in contract contexts:

```bash
cargo test -p security-disclaimers --test integration_tests
```

### Contract Testing

Test security disclaimer integration in your contracts:

```rust
#[test]
fn test_security_disclaimer_integration() {
    let env = Env::default();
    
    // Test disclaimer retrieval
    let disclaimer = YourContract::get_security_disclaimer(&env, DisclaimerCategory::Audit);
    assert!(disclaimer.contains("SECURITY WARNING"));
    
    // Test security validation
    let is_secure = YourContract::validate_security_config(&env, true, false);
    assert!(is_secure);
}
```

## 📅 Security Checklist

### Pre-Deployment

- [ ] Determine appropriate security level for your contract
- [ ] Add security disclaimer dependency
- [ ] Implement security disclaimer functions
- [ ] Add security documentation to contract
- [ ] Run security validation tests
- [ ] Verify audit requirements are met

### Post-Deployment

- [ ] Monitor security disclaimer events
- [ ] Validate security configuration remains valid
- [ ] Update disclaimers if contract evolves
- [ ] Conduct regular security reviews

## 🚨 Security Best Practices

### 1. Choose Appropriate Security Level

- **Critical**: Only for contracts handling significant value or with complex governance
- **High**: For contracts with user funds or sensitive operations
- **Medium**: For contracts with limited risk exposure
- **Low**: For utility contracts with minimal risk

### 2. Implement Security Validation

```rust
pub fn sensitive_operation(env: Env, caller: Address) {
    // Always validate security configuration
    let is_secure = Self::validate_security_config(&env, has_admin, has_upgrade);
    if !is_secure {
        env.panic_with_error(Error::SecurityCheckFailed);
    }
    
    // Continue with operation
}
```

### 3. Emit Security Events

```rust
pub fn upgrade_contract(env: Env, new_impl: BytesN<32>) {
    let disclaimer = Self::get_security_disclaimer(&env, DisclaimerCategory::Upgrade);
    env.events().publish((Symbol::short!("security_warning"),), disclaimer);
    
    // Perform upgrade
}
```

### 4. Document Security Considerations

Add specific security considerations to your contract documentation:

```rust
//! ## Security Considerations
//!
//! - This contract handles user funds and requires careful security review
//! - All admin operations should be protected by multi-signature
//! - Monitor for unusual transaction patterns
//! - Consider implementing time delays for critical operations
```

## 🔍 Monitoring and Maintenance

### Security Disclaimer Monitoring

Monitor security disclaimer usage in your contracts:

```rust
pub fn get_security_status(env: Env) -> SecurityStatus {
    let audit_required = SecurityDisclaimer::requires_audit(&env, SecurityLevel::High);
    let config_valid = Self::validate_security_config(&env, true, false);
    
    SecurityStatus {
        audit_required,
        config_valid,
        last_check: env.ledger().timestamp(),
    }
}
```

### Regular Updates

- Review security levels annually or after major changes
- Update disclaimers when contract functionality changes
- Reassess audit requirements based on contract evolution

## 📚 Reference

### API Reference

#### SecurityDisclaimer::get_disclaimer
```rust
pub fn get_disclaimer(env: Env, level: SecurityLevel, category: DisclaimerCategory) -> String
```
Returns the appropriate security disclaimer for the given level and category.

#### SecurityDisclaimer::requires_audit
```rust
pub fn requires_audit(env: Env, level: SecurityLevel) -> bool
```
Returns whether the given security level requires an audit.

#### SecurityDisclaimer::validate_security_config
```rust
pub fn validate_security_config(env: Env, level: SecurityLevel, has_admin: bool, has_upgrade: bool) -> bool
```
Validates if the security configuration meets requirements for the given level.

#### SecurityDisclaimer::get_testing_requirements
```rust
pub fn get_testing_requirements(env: Env, level: SecurityLevel) -> String
```
Returns testing requirements for the given security level.

### Macros

#### security_disclaimer!
```rust
security_disclaimer!(SecurityLevel::High)
```
Convenience macro for generating security disclaimer text.

### Types

#### SecurityLevel
- `Low = 0`
- `Medium = 1` 
- `High = 2`
- `Critical = 3`

#### DisclaimerCategory
- `Audit = 0`
- `Usage = 1`
- `Upgrade = 2`
- `Emergency = 3`

## 🤝 Contributing

When adding new contracts or modifying existing ones:

1. Determine the appropriate security level
2. Add security disclaimer functions
3. Include security documentation
4. Add appropriate tests
5. Update this guide if needed

## 📞 Support

For questions about security disclaimers:

- Check the unit tests in `contracts/security-disclaimers/tests/`
- Review integration tests for usage examples
- Consult the contract implementations in `contracts/*/src/lib.rs`
- Open an issue for questions or feature requests

---

**Last Updated:** April 28, 2026  
**Version:** 1.0  
**Status:** Production Ready
