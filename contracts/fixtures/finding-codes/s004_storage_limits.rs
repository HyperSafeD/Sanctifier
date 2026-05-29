#![no_std]
use soroban_sdk::{contract, contractimpl, Bytes, Env};

#[contract]
pub struct StorageLimitFixture;

#[contractimpl]
impl StorageLimitFixture {
    pub fn write_large_blob(env: Env, payload: Bytes) {
        env.storage().instance().set(&"large_payload", &payload);
    }
}
