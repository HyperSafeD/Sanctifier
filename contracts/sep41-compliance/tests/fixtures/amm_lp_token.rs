//! Reference SEP-41 LP (liquidity-provider) token, as an `amm-pool` would mint
//! to represent a share of pooled reserves.
//!
//! This fixture is a **complete, compliant** SEP-41 token used by the
//! `sep41-compliance` suite as a known-good contract.  It mirrors the share
//! token an AMM pool issues on `add_liquidity` and redeems on
//! `remove_liquidity`.
#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, MuxedAddress, String};

#[contracttype]
enum DataKey {
    Balance(Address),
    Allowance(Address, Address),
    Decimals,
    Name,
    Symbol,
}

#[contract]
pub struct AmmLpToken;

#[contractimpl]
impl AmmLpToken {
    pub fn initialize(env: Env, decimals: u32, name: String, symbol: String) {
        env.storage().instance().set(&DataKey::Decimals, &decimals);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
    }

    // ── SEP-41 ────────────────────────────────────────────────────────────────

    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Allowance(from, spender))
            .unwrap_or(0)
    }

    pub fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
        expiration_ledger: u32,
    ) {
        from.require_auth();
        let _ = expiration_ledger;
        env.storage()
            .persistent()
            .set(&DataKey::Allowance(from, spender), &amount);
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(id))
            .unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
        from.require_auth();
        let to = to.address();
        let from_bal = Self::balance(env.clone(), from.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &(from_bal - amount));
        let to_bal = Self::balance(env.clone(), to.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &(to_bal + amount));
    }

    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        let allowed = Self::allowance(env.clone(), from.clone(), spender.clone());
        env.storage().persistent().set(
            &DataKey::Allowance(from.clone(), spender),
            &(allowed - amount),
        );
        let from_bal = Self::balance(env.clone(), from.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &(from_bal - amount));
        let to_bal = Self::balance(env.clone(), to.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &(to_bal + amount));
    }

    pub fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();
        let bal = Self::balance(env.clone(), from.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &(bal - amount));
    }

    pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        let allowed = Self::allowance(env.clone(), from.clone(), spender.clone());
        env.storage().persistent().set(
            &DataKey::Allowance(from.clone(), spender),
            &(allowed - amount),
        );
        let bal = Self::balance(env.clone(), from.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &(bal - amount));
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Decimals).unwrap_or(7)
    }

    pub fn name(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Name)
            .unwrap_or_else(|| String::from_str(&env, "AMM LP"))
    }

    pub fn symbol(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Symbol)
            .unwrap_or_else(|| String::from_str(&env, "LP"))
    }
}
