/// Fixture for S008 event analysis tests.
///
/// Contains deliberate event emission defects:
/// 1. Inconsistent topic counts for the same event name across two functions.
/// 2. String-literal topic that could use `symbol_short!` for gas savings.
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

#[contract]
pub struct EventContract;

#[contractimpl]
impl EventContract {
    /// Emits "transfer" with 2 topics.
    pub fn transfer_v1(env: Env) {
        env.events().publish(("transfer", symbol_short!("from")), 100i128);
    }

    /// Emits "transfer" with 3 topics — inconsistent with transfer_v1.
    pub fn transfer_v2(env: Env) {
        env.events()
            .publish(("transfer", symbol_short!("from"), symbol_short!("to")), 200i128);
    }

    /// Uses a raw string literal topic instead of symbol_short! — gas sub-optimal.
    pub fn minted(env: Env) {
        env.events().publish(("mint", "amount"), 500i128);
    }
}
