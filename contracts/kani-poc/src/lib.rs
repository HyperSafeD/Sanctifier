#![no_std]

//! Proof-of-concept: Kani formal verification harnesses for a standard Soroban token contract.
//!
//! This module demonstrates the "Core Logic Separation" pattern: extract pure balance/transfer
//! logic into functions that can be verified with Kani, while the contract layer that uses
//! `Env`, `Address`, `Symbol`, etc. remains unverified due to Host type limitations.

use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

// ── Pure logic (verified with Kani) ─────────────────────────────────────────────
//
// These functions operate only on i128 and have no Host/FFI dependencies.
// They model the core arithmetic of a standard Soroban token: transfer, mint, burn.

/// Transfer: deduct from sender, add to receiver.
pub fn transfer_pure(
    balance_from: i128,
    balance_to: i128,
    amount: i128,
) -> Result<(i128, i128), &'static str> {
    if amount <= 0 {
        return Err("Amount must be positive");
    }
    let new_from = balance_from
        .checked_sub(amount)
        .ok_or("Insufficient balance")?;
    let new_to = balance_to
        .checked_add(amount)
        .ok_or("Receiver balance overflow")?;
    Ok((new_from, new_to))
}

/// Mint: add to a balance.
pub fn mint_pure(balance: i128, amount: i128) -> Result<i128, &'static str> {
    if amount <= 0 {
        return Err("Mint amount must be positive");
    }
    balance.checked_add(amount).ok_or("Mint overflow")
}

/// Burn: subtract from a balance.
pub fn burn_pure(balance: i128, amount: i128) -> Result<i128, &'static str> {
    if amount <= 0 {
        return Err("Burn amount must be positive");
    }
    balance
        .checked_sub(amount)
        .ok_or("Insufficient balance to burn")
}

// ── Contract (not verified: uses Host types) ────────────────────────────────────

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    /// Wrapper exposing transfer_pure for contract use.
    /// A full implementation would read/write balances via env.storage().
    pub fn transfer(balance_from: i128, balance_to: i128, amount: i128) -> (i128, i128) {
        transfer_pure(balance_from, balance_to, amount).expect("transfer failed")
    }

    /// A function that interacts with Env (Host types).
    /// Kani cannot verify this: Env, Symbol, and storage operations require host FFI.
    pub fn set_admin(env: Env, new_admin: Symbol) {
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &new_admin);
    }
}

// ── Kani harnesses ─────────────────────────────────────────────────────────────

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn verify_transfer_pure_conservation() {
        let balance_from: i128 = kani::any();
        let balance_to: i128 = kani::any();
        let amount: i128 = kani::any();

        kani::assume(amount > 0);
        kani::assume(balance_from >= amount);
        kani::assume(balance_from <= i128::MAX);
        kani::assume(balance_to >= 0);
        kani::assume(balance_to <= i128::MAX - amount);

        let Ok((new_from, new_to)) = transfer_pure(balance_from, balance_to, amount) else {
            kani::unreachable();
        };

        assert!(new_from == balance_from - amount);
        assert!(new_to == balance_to + amount);
        assert!(
            new_from + new_to == balance_from + balance_to,
            "Conservation of supply"
        );
    }

    #[kani::proof]
    fn verify_transfer_pure_insufficient_balance() {
        let balance_from: i128 = kani::any();
        let balance_to: i128 = kani::any();
        let amount: i128 = kani::any();

        kani::assume(amount > 0);
        kani::assume(balance_from < amount);

        let result = transfer_pure(balance_from, balance_to, amount);
        assert!(result.is_err());
    }

    #[kani::proof]
    fn verify_mint_pure() {
        let balance: i128 = kani::any();
        let amount: i128 = kani::any();

        kani::assume(amount > 0);
        kani::assume(balance >= 0);
        kani::assume(balance <= i128::MAX - amount);

        let Ok(new_balance) = mint_pure(balance, amount) else {
            kani::unreachable();
        };

        assert!(new_balance == balance + amount);
    }

    #[kani::proof]
    fn verify_burn_pure() {
        let balance: i128 = kani::any();
        let amount: i128 = kani::any();

        kani::assume(amount > 0);
        kani::assume(balance >= amount);

        let Ok(new_balance) = burn_pure(balance, amount) else {
            kani::unreachable();
        };

        assert!(new_balance == balance - amount);
    }
}
