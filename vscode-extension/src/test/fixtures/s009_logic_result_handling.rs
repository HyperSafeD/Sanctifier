#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct ResultHandlingFixture;

#[contractimpl]
impl ResultHandlingFixture {
    pub fn ignore_result(_env: Env) {
        let _ = parse_number();
    }
}

fn parse_number() -> Result<u32, ()> {
    Err(())
}
