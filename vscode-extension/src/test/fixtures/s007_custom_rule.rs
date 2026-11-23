#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct CustomRuleFixture;

#[contractimpl]
impl CustomRuleFixture {
    pub fn todo_pattern(_env: Env) {
        let _marker = "TODO: replace debug bypass";
    }
}
