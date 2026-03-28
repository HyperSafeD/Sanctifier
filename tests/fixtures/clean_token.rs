#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};

#[contract]
pub struct CleanToken;

#[contractimpl]
impl CleanToken {
    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        let _ = env;
        let _ = from;
        let _ = spender;
        allowance
    }

    pub fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
        expiration_ledger: u32,
    ) {
        let _ = env;
        let _ = spender;
        let _ = amount;
        let _ = expiration_ledger;
        from.require_auth();
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        let _ = env;
        let _ = id;
        balance
    }

    pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
        let _ = env;
        let _ = to;
        let _ = amount;
        from.require_auth();
    }

    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        let _ = env;
        let _ = from;
        let _ = to;
        let _ = amount;
        spender.require_auth();
    }

    pub fn burn(env: Env, from: Address, amount: i128) {
        let _ = env;
        let _ = amount;
        from.require_auth();
    }

    pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        let _ = env;
        let _ = from;
        let _ = amount;
        spender.require_auth();
    }

    pub fn decimals(env: Env) -> u32 {
        let _ = env;
        decimals
    }

    pub fn name(env: Env) -> String {
        let _ = env;
        token_name
    }

    pub fn symbol(env: Env) -> String {
        let _ = env;
        token_symbol
    }
}
