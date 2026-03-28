#![cfg(test)]
#![allow(unexpected_cfgs)]

use runtime_guard_wrapper::RuntimeGuardWrapper;
use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, vec, Address, Env, IntoVal, Symbol, Val, Vec,
};

#[contract]
pub struct RuntimeGuardWrapperHarness;

#[contractimpl]
impl RuntimeGuardWrapperHarness {
    pub fn init(env: Env, wrapped_contract: Address) {
        RuntimeGuardWrapper::init(env, wrapped_contract)
    }

    pub fn get_wrapped_contract(env: Env) -> Address {
        RuntimeGuardWrapper::get_wrapped_contract(env)
    }

    pub fn execute_guarded(
        env: Env,
        function_name: Symbol,
        args: Vec<Val>,
    ) -> Result<Val, soroban_sdk::Error> {
        RuntimeGuardWrapper::execute_guarded(env, function_name, args)
    }

    pub fn get_stats(env: Env) -> (u32, u32, u32) {
        RuntimeGuardWrapper::get_stats(env)
    }

    pub fn health_check(env: Env) -> bool {
        RuntimeGuardWrapper::health_check(env)
    }
}

fn setup(env: &Env) -> (RuntimeGuardWrapperHarnessClient<'_>, Address) {
    let contract_id = env.register_contract(None, RuntimeGuardWrapperHarness);
    let wrapped = Address::generate(&env);
    let client = RuntimeGuardWrapperHarnessClient::new(&env, &contract_id);
    client.init(&wrapped);
    (client, wrapped)
}

#[test]
fn execute_guarded_rejects_missing_function_name() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let result = client.try_execute_guarded(&Symbol::new(&env, "missing"), &vec![&env]);

    assert!(result.is_err());
    assert_eq!(client.get_stats(), (0, 0, 0));
}

#[test]
fn execute_guarded_rejects_argument_count_mismatch() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let args = vec![&env, 7u32.into_val(&env)];
    let result = client.try_execute_guarded(&Symbol::new(&env, "ping"), &args);

    assert!(result.is_err());
    assert_eq!(client.get_stats(), (0, 0, 0));
}

#[test]
fn init_called_twice_is_idempotent() {
    let env = Env::default();
    let (client, wrapped) = setup(&env);
    let replacement = Address::generate(&env);

    client.init(&replacement);

    assert_eq!(client.get_wrapped_contract(), wrapped);
    assert_eq!(client.get_stats(), (0, 0, 0));
}

#[test]
fn health_check_fails_after_storage_budget_is_exhausted() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let mut index = 0u32;
    while index < 64 {
        let _ = client.execute_guarded(&Symbol::new(&env, "ping"), &vec![&env]);
        index = index.saturating_add(1);
    }

    assert!(!client.health_check());
}

#[test]
fn get_stats_tracks_successes_and_failures() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let empty = vec![&env];

    let _ = client.execute_guarded(&Symbol::new(&env, "ping"), &empty);
    let _ = client.execute_guarded(&Symbol::new(&env, "echo"), &vec![&env, 9u32.into_val(&env)]);
    let _ = client.execute_guarded(
        &Symbol::new(&env, "sum"),
        &vec![&env, 2u32.into_val(&env), 3u32.into_val(&env)],
    );
    let missing = client.try_execute_guarded(&Symbol::new(&env, "missing"), &empty);
    let mismatch =
        client.try_execute_guarded(&Symbol::new(&env, "ping"), &vec![&env, 1u32.into_val(&env)]);

    assert!(missing.is_err());
    assert!(mismatch.is_err());

    assert_eq!(client.get_stats(), (3, 3, 0));
}
