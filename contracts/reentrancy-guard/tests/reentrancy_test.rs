#![no_std]

use reentrancy_guard::{enter_pure, GuardStatus, ReentrancyGuard};
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct ProtectedContract;

#[contractimpl]
impl ProtectedContract {
    pub fn do_something(env: Env) {
        let guard = ReentrancyGuard::new(&env);
        guard.enter();
        // Section protected from reentrancy
        guard.exit();
    }

    pub fn malicious_reentry(env: Env) {
        let guard = ReentrancyGuard::new(&env);
        guard.enter();
        // Maliciously call back into do_something
        Self::do_something(env.clone());
        guard.exit();
    }
}

#[test]
fn test_reentrancy_protection() {
    let result = enter_pure(GuardStatus::Locked);

    assert!(result.is_err());
}

#[test]
fn test_normal_usage() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProtectedContract);
    let client = ProtectedContractClient::new(&env, &contract_id);

    client.do_something();
    client.do_something(); // Sequential calls should work
}
