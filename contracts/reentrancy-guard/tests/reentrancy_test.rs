#![no_std]

use reentrancy_guard::{enter_pure, GuardStatus, ReentrancyGuard};
use soroban_sdk::{contract, contractimpl, Env, String};

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

    pub fn seed_legacy_guard_key(env: Env, value: u32) {
        env.storage()
            .instance()
            .set(&String::from_str(&env, "Guard"), &value);
    }

    pub fn legacy_guard_key(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&String::from_str(&env, "Guard"))
            .unwrap_or(0)
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

#[test]
fn typed_storage_key_does_not_collide_with_legacy_string_key() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProtectedContract);
    let client = ProtectedContractClient::new(&env, &contract_id);

    client.seed_legacy_guard_key(&77);
    client.do_something();

    assert_eq!(client.legacy_guard_key(), 77);
    client.do_something();
}
