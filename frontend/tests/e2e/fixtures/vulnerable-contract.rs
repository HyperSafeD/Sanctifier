#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct VulnerableContract;

#[contractimpl]
impl VulnerableContract {
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        let _ = env;
        let _ = from;
        let _ = to;

        if amount < 0 {
            panic!("negative transfer");
        }
    }
}
