#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct TokenInterfaceFixture;

#[contractimpl]
impl TokenInterfaceFixture {
    pub fn transfer(_env: Env, _from: Address, _to: Address, _amount: i128) {
        // Intentionally minimal fixture for interface checks.
    }
}
