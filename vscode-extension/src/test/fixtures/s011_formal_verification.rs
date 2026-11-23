#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct FormalVerificationFixture;

#[contractimpl]
impl FormalVerificationFixture {
    pub fn invariant_candidate(_env: Env, total_supply: i128, reserves: i128) -> bool {
        reserves >= total_supply
    }
}
