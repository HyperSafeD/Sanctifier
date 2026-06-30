//! Reference SEP-41 deposit-receipt token, as a `deposit-withdraw` vault would
//! mint to represent a depositor's claim on the underlying balance.
//!
//! Complete and compliant — used by the `sep41-compliance` suite as a
//! known-good contract. Uses `require_auth_for_args` on `transfer` to exercise
//! the harness's recognition of that authorization variant.
#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, IntoVal, MuxedAddress, String,
};

#[contracttype]
enum DataKey {
    Balance(Address),
    Allowance(Address, Address),
}

#[contract]
pub struct DepositReceiptToken;

#[contractimpl]
impl DepositReceiptToken {
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
        // Exercise the `require_auth_for_args` authorization variant.
        from.require_auth_for_args((&from, &amount).into_val(&env));
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

    pub fn decimals(_env: Env) -> u32 {
        7
    }

    pub fn name(env: Env) -> String {
        String::from_str(&env, "Deposit Receipt")
    }

    pub fn symbol(env: Env) -> String {
        String::from_str(&env, "dRCPT")
    }
}
