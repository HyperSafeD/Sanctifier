#![no_std]
//! S012 Test Fixture: SEP-41 Token Interface Compliance
//!
//! This fixture demonstrates various S012 (SEP-41 interface) violations:
//! 1. MissingFunction: Required SEP-41 functions not present
//! 2. SignatureMismatch: Function exists but signature is wrong
//! 3. AuthorizationMismatch: Function exists but lacks proper authorization
//!
//! CI validates that S012 findings are detected in this fixture.
//! See: .github/workflows/ci.yml and tooling/sanctifier-core/tests/sep41_tests.rs

use soroban_sdk::{contract, contractimpl, Address, Env, String};

#[contract]
pub struct TokenInterfaceFixture;

#[contractimpl]
impl TokenInterfaceFixture {
    /// ❌ SignatureMismatch: Should use MuxedAddress for 'to' parameter (3rd param)
    /// Expected: transfer(env: Env, from: Address, to: MuxedAddress, amount: i128)
    pub fn transfer(_env: Env, _from: Address, _to: Address, _amount: i128) {
        // Intentionally minimal fixture for interface checks.
        // Also missing: _from.require_auth() (but we'll catch signature first)
    }
    
    /// ❌ AuthorizationMismatch: Missing from.require_auth()
    /// Expected: from.require_auth() at the start of the function
    pub fn approve(_env: Env, _from: Address, _spender: Address, _amount: i128, _expiration_ledger: u32) {
        // No authorization - violates SEP-41 spec!
    }
    
    /// ✅ Correct: balance doesn't require authorization
    pub fn balance(_env: Env, _id: Address) -> i128 {
        0
    }
    
    // ❌ MissingFunction: Missing the following required functions:
    // - allowance
    // - transfer_from
    // - burn
    // - burn_from
    // - decimals
    // - name
    // - symbol
    //
    // These omissions should trigger MissingFunction findings in CI.
}
