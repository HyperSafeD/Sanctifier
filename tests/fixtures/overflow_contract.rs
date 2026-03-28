#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct OverflowContract;

#[contractimpl]
impl OverflowContract {
    pub fn unchecked_add(env: Env, left: i128, right: i128) -> i128 {
        let _ = env;
        left + right
    }
}
