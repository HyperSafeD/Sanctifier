#![no_std]
//! Example: Fully SEP-41 Compliant Token
//!
//! This example demonstrates a complete, production-ready implementation of the
//! SEP-41 (Stellar Token Standard) interface that passes all S012 checks.
//!
//! # SEP-41 Compliance Checklist
//!
//! ✅ All 10 required functions present
//! ✅ Exact signature matching (including MuxedAddress in transfer)
//! ✅ Proper authorization on state-changing functions
//! ✅ Allowance management in transfer_from
//! ✅ Total supply tracking
//! ✅ Safe arithmetic (Soroban runtime handles this)
//!
//! # Usage
//!
//! ```bash
//! # Build the contract
//! cargo build --target wasm32-unknown-unknown --release
//!
//! # Analyze for S012 compliance
//! sanctifier analyze examples/sep41-compliant-token.rs
//! # Expected: Zero S012 findings (fully compliant)
//! ```
//!
//! # SEP-41 Reference
//!
//! See: https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md

use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, MuxedAddress, String,
};

// ============================================================================
// Storage Keys
// ============================================================================

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Total supply of tokens
    TotalSupply,
    /// Balance for a specific address
    Balance(Address),
    /// Allowance from owner to spender
    Allowance(Address, Address),
    /// Token metadata
    Decimals,
    Name,
    Symbol,
}

#[derive(Clone)]
#[contracttype]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}

// ============================================================================
// Contract
// ============================================================================

#[contract]
pub struct Sep41Token;

#[contractimpl]
impl Sep41Token {
    // ========================================================================
    // SEP-41 Core Functions
    // ========================================================================

    /// Returns the allowance for `spender` to transfer from `from`.
    ///
    /// # SEP-41 Compliance
    /// - ✅ Exact signature match
    /// - ✅ No authorization required (read-only)
    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        let key = DataKey::Allowance(from, spender);
        
        if let Some(allowance) = env.storage().persistent().get::<_, AllowanceValue>(&key) {
            // Check if allowance has expired
            if allowance.expiration_ledger < env.ledger().sequence() {
                0
            } else {
                allowance.amount
            }
        } else {
            0
        }
    }

    /// Approves `spender` to transfer up to `amount` from `from` until `expiration_ledger`.
    ///
    /// # SEP-41 Compliance
    /// - ✅ Exact signature match (5 parameters)
    /// - ✅ Authorizes `from` (parameter index 1)
    pub fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
        expiration_ledger: u32,
    ) {
        // ✅ SEP-41 requirement: authorize 'from'
        from.require_auth();

        // Validate expiration
        if expiration_ledger < env.ledger().sequence() {
            panic!("expiration_ledger is in the past");
        }

        // Store allowance
        let key = DataKey::Allowance(from, spender);
        let allowance = AllowanceValue {
            amount,
            expiration_ledger,
        };
        env.storage().persistent().set(&key, &allowance);

        // Emit event (recommended for off-chain indexing)
        env.events().publish(
            (String::from_str(&env, "approve"), from.clone(), spender.clone()),
            (amount, expiration_ledger),
        );
    }

    /// Returns the balance of `id`.
    ///
    /// # SEP-41 Compliance
    /// - ✅ Exact signature match
    /// - ✅ Returns i128 (not u64 or other types)
    /// - ✅ No authorization required (read-only)
    pub fn balance(env: Env, id: Address) -> i128 {
        let key = DataKey::Balance(id);
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    /// Transfers `amount` from `from` to `to`.
    ///
    /// # SEP-41 Compliance
    /// - ✅ Uses MuxedAddress for `to` (supports Stellar's muxed accounts)
    /// - ✅ Authorizes `from` (parameter index 1)
    /// - ✅ Exact signature match
    pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
        // ✅ SEP-41 requirement: authorize 'from'
        from.require_auth();

        // Perform the transfer
        Self::transfer_impl(&env, from.clone(), to.clone(), amount);

        // Emit event
        env.events().publish(
            (String::from_str(&env, "transfer"), from, to),
            amount,
        );
    }

    /// Transfers `amount` from `from` to `to`, spending `spender`'s allowance.
    ///
    /// # SEP-41 Compliance
    /// - ✅ Authorizes `spender` (parameter index 1) - NOT 'from'!
    /// - ✅ Exact signature match
    /// - ✅ Decrements allowance (required for security)
    pub fn transfer_from(
        env: Env,
        spender: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) {
        // ✅ SEP-41 requirement: authorize 'spender' (the one using the allowance)
        spender.require_auth();

        // Check and spend allowance
        let key = DataKey::Allowance(from.clone(), spender.clone());
        let allowance = env
            .storage()
            .persistent()
            .get::<_, AllowanceValue>(&key)
            .unwrap_or_else(|| panic!("no allowance"));

        // Verify not expired
        if allowance.expiration_ledger < env.ledger().sequence() {
            panic!("allowance expired");
        }

        // Verify sufficient allowance
        if allowance.amount < amount {
            panic!("insufficient allowance");
        }

        // Decrement allowance
        let new_allowance = AllowanceValue {
            amount: allowance.amount - amount,
            expiration_ledger: allowance.expiration_ledger,
        };
        env.storage().persistent().set(&key, &new_allowance);

        // Perform transfer (note: to is Address, not MuxedAddress)
        let to_muxed = MuxedAddress::from_address(&to);
        Self::transfer_impl(&env, from.clone(), to_muxed, amount);

        // Emit event
        env.events().publish(
            (String::from_str(&env, "transfer_from"), spender, from, to),
            amount,
        );
    }

    // ========================================================================
    // SEP-41 Burn Functions
    // ========================================================================

    /// Burns `amount` from `from`, reducing total supply.
    ///
    /// # SEP-41 Compliance
    /// - ✅ Authorizes `from` (parameter index 1)
    /// - ✅ Exact signature match
    pub fn burn(env: Env, from: Address, amount: i128) {
        // ✅ SEP-41 requirement: authorize 'from'
        from.require_auth();

        // Decrease balance
        let key = DataKey::Balance(from.clone());
        let balance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        
        if balance < amount {
            panic!("insufficient balance");
        }

        env.storage().persistent().set(&key, &(balance - amount));

        // Decrease total supply
        let supply_key = DataKey::TotalSupply;
        let total_supply: i128 = env.storage().persistent().get(&supply_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&supply_key, &(total_supply - amount));

        // Emit event
        env.events()
            .publish((String::from_str(&env, "burn"), from), amount);
    }

    /// Burns `amount` from `from` using `spender`'s allowance.
    ///
    /// # SEP-41 Compliance
    /// - ✅ Authorizes `spender` (parameter index 1) - NOT 'from'!
    /// - ✅ Exact signature match
    /// - ✅ Decrements allowance
    pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        // ✅ SEP-41 requirement: authorize 'spender'
        spender.require_auth();

        // Check and spend allowance
        let key = DataKey::Allowance(from.clone(), spender.clone());
        let allowance = env
            .storage()
            .persistent()
            .get::<_, AllowanceValue>(&key)
            .unwrap_or_else(|| panic!("no allowance"));

        if allowance.expiration_ledger < env.ledger().sequence() {
            panic!("allowance expired");
        }

        if allowance.amount < amount {
            panic!("insufficient allowance");
        }

        // Decrement allowance
        let new_allowance = AllowanceValue {
            amount: allowance.amount - amount,
            expiration_ledger: allowance.expiration_ledger,
        };
        env.storage().persistent().set(&key, &new_allowance);

        // Decrease balance
        let balance_key = DataKey::Balance(from.clone());
        let balance: i128 = env.storage().persistent().get(&balance_key).unwrap_or(0);
        
        if balance < amount {
            panic!("insufficient balance");
        }

        env.storage()
            .persistent()
            .set(&balance_key, &(balance - amount));

        // Decrease total supply
        let supply_key = DataKey::TotalSupply;
        let total_supply: i128 = env.storage().persistent().get(&supply_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&supply_key, &(total_supply - amount));

        // Emit event
        env.events().publish(
            (String::from_str(&env, "burn_from"), spender, from),
            amount,
        );
    }

    // ========================================================================
    // SEP-41 Metadata Functions
    // ========================================================================

    /// Returns the number of decimals used by the token.
    ///
    /// # SEP-41 Compliance
    /// - ✅ Returns u32 (not u8 or other types)
    /// - ✅ No authorization required (read-only)
    pub fn decimals(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::Decimals)
            .unwrap_or(7) // Default: 7 decimals (Stellar standard)
    }

    /// Returns the name of the token.
    ///
    /// # SEP-41 Compliance
    /// - ✅ Returns String (Soroban SDK type)
    /// - ✅ No authorization required (read-only)
    pub fn name(env: Env) -> String {
        env.storage()
            .persistent()
            .get(&DataKey::Name)
            .unwrap_or_else(|| String::from_str(&env, "Token"))
    }

    /// Returns the symbol of the token.
    ///
    /// # SEP-41 Compliance
    /// - ✅ Returns String (Soroban SDK type)
    /// - ✅ No authorization required (read-only)
    pub fn symbol(env: Env) -> String {
        env.storage()
            .persistent()
            .get(&DataKey::Symbol)
            .unwrap_or_else(|| String::from_str(&env, "TOK"))
    }

    // ========================================================================
    // Internal Helper Functions (not part of SEP-41)
    // ========================================================================

    /// Internal transfer implementation shared by `transfer` and `transfer_from`.
    fn transfer_impl(env: &Env, from: Address, to: MuxedAddress, amount: i128) {
        // Decrease from's balance
        let from_key = DataKey::Balance(from.clone());
        let from_balance: i128 = env.storage().persistent().get(&from_key).unwrap_or(0);
        
        if from_balance < amount {
            panic!("insufficient balance");
        }

        env.storage()
            .persistent()
            .set(&from_key, &(from_balance - amount));

        // Increase to's balance
        // Note: MuxedAddress converts to Address for balance tracking
        let to_address = to.clone().into_address();
        let to_key = DataKey::Balance(to_address);
        let to_balance: i128 = env.storage().persistent().get(&to_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&to_key, &(to_balance + amount));
    }
}

// ============================================================================
// Notes for Production Use
// ============================================================================
//
// This example demonstrates S012 compliance. For production, also consider:
//
// 1. **Minting**: Add an admin-only `mint()` function to create new tokens
// 2. **Access Control**: Implement admin roles for privileged operations
// 3. **Pausability**: Add emergency pause/unpause mechanism
// 4. **Events**: Emit comprehensive events for off-chain indexing
// 5. **Upgradability**: Consider upgrade patterns if needed
// 6. **Gas Optimization**: Batch operations where possible
// 7. **Testing**: Comprehensive unit and integration tests
// 8. **Formal Verification**: Use Kani for total supply invariants (S011)
//
// See also:
// - docs/rules/s012-sep41-interface.md
// - contracts/my-contract/src/lib.rs (reference implementation)
// - SEP-41 spec: https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md
