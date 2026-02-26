#![no_std]

use soroban_sdk::{contract, contractimpl, Env, testutils::Address as _};
use reentrancy_guard::ReentrancyGuard;

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
#[should_panic(expected = "reentrancy detected")]
fn test_reentrancy_protection() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProtectedContract);
    let client = ProtectedContractClient::new(&env, &contract_id);
    
    client.malicious_reentry();
}

#[test]
fn test_normal_usage() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProtectedContract);
    let client = ProtectedContractClient::new(&env, &contract_id);
    
    client.do_something();
    client.do_something(); // Sequential calls should work
}
