#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

#[contract]
pub struct StorageKeyFixture;

#[contractimpl]
impl StorageKeyFixture {
    pub fn write_mixed_keys(env: Env, user_key: Symbol, value: Symbol) {
        env.storage().instance().set(&symbol_short!("DATA"), &value);
        env.storage().instance().set(&user_key, &value);
    }
}
