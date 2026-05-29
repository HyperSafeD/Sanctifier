#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env};

#[contract]
pub struct UnsafePrngFixture;

#[contractimpl]
impl UnsafePrngFixture {
    pub fn unstable_bet(env: Env) -> u32 {
        // ❌ VULNERABLE: Using ledger timestamp as source of randomness.
        // This is predictable by miners/validators.
        let ts = env.ledger().timestamp();
        (ts % 100) as u32
    }
}
