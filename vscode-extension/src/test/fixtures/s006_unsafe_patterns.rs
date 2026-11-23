#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct UnsafePatternFixture;

#[contractimpl]
impl UnsafePatternFixture {
    pub fn low_level_pointer(_env: Env, pointer_like: u64) -> u64 {
        pointer_like
    }
}
