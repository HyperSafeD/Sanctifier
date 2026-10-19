#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

#[contract]
pub struct AuthGapFixture;

#[contractimpl]
impl AuthGapFixture {
    pub fn set_owner(env: Env, owner: Symbol) {
        env.storage().instance().set(&symbol_short!("OWNER"), &owner);
    }
}
