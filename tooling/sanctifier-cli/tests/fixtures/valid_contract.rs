#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Address};

#[contract]
pub struct ValidContract;

#[contractimpl]
impl ValidContract {
    pub fn do_nothing(env: Env, admin: Address) {
        // Safe access
    }

    pub fn safe_calc(env: Env, a: u64, b: u64) -> u64 {
        a.checked_add(b).unwrap_or(0)
    }
}
