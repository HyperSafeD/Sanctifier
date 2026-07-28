#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct PoolContract;

#[contractimpl]
impl PoolContract {
    // Bad: No auth
    pub fn withdraw_reserve(env: Env, to: Address, amount: i128) {
        let token = get_token(&env);
        token.transfer(&env.current_contract_address(), &to, &amount);
    }

    // Bad: Weak auth (checking simple signatures instead of `require_auth` or proper nonce check)
    pub fn treasury_transfer(env: Env, to: Address, amount: i128, signature: BytesN<64>) {
        // Missing require_auth or robust nonce validation
        let token = get_token(&env);
        token.transfer(&env.current_contract_address(), &to, &amount);
    }

    // Good: Proper auth
    pub fn withdraw_treasury(env: Env, admin: Address, to: Address, amount: i128) {
        admin.require_auth();
        let token = get_token(&env);
        token.transfer(&env.current_contract_address(), &to, &amount);
    }
    
    // Good: nonce validation
    pub fn withdraw_reserve_with_nonce(env: Env, admin: Address, to: Address, amount: i128, nonce: i128) {
        verify_admin_and_nonce(&env, &admin, nonce);
        let token = get_token(&env);
        token.transfer(&env.current_contract_address(), &to, &amount);
    }
}
