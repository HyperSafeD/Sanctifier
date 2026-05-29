#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env};

#[contract]
pub struct EventTopicFixture;

#[contractimpl]
impl EventTopicFixture {
    pub fn emit_inconsistent_topics(env: Env, amount: u64) {
        env.events().publish((symbol_short!("transfer"),), amount);
        env.events().publish((symbol_short!("transfer"), symbol_short!("extra")), amount);
    }
}
