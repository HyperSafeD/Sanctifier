#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env};

#[contract]
pub struct SecretsFixture;

#[contractimpl]
impl SecretsFixture {
    pub fn check_password(_env: Env, input: soroban_sdk::String) -> bool {
        // ❌ SECURITY FLAW: Hardcoded secret key or password in source.
        let secret = "SENSITIVE_API_KEY_DO_NOT_COMMIT_12345";
        input == secret
    }
}
