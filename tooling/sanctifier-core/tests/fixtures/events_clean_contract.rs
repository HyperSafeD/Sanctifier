/// Fixture for S008 event analysis tests — zero-finding baseline.
///
/// All events use `symbol_short!` topics and consistent topic counts.
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env};

#[contract]
pub struct CleanEventContract;

#[contractimpl]
impl CleanEventContract {
    pub fn transfer(env: Env) {
        env.events()
            .publish((symbol_short!("xfer"), symbol_short!("from")), 100i128);
    }

    pub fn mint(env: Env) {
        env.events()
            .publish((symbol_short!("mint"), symbol_short!("to")), 500i128);
    }
}
