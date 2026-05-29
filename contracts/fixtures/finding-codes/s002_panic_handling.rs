#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct PanicFixture;

#[contractimpl]
impl PanicFixture {
    pub fn unwrap_like(_env: Env, items: [u32; 1], index: u32) -> u32 {
        let maybe = items.get(index as usize);
        maybe.expect("index out of range")
    }
}
