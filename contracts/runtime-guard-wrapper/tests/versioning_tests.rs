//! Contract versioning convention tests for RuntimeGuardWrapper.
//!
//! Fixtures
//! --------
//! * `fixture_uninitialized` – registered contract with no `init` called yet
//! * `fixture_initialized`   – contract after `init`; version key stamped as 1
//! * `fixture_active`        – initialized contract with several guarded calls recorded

#![cfg(test)]
#![allow(unexpected_cfgs)]

use runtime_guard_wrapper::{RuntimeGuardWrapper, CONTRACT_VERSION};
use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, vec, Address, Env, IntoVal, Symbol, Val, Vec,
};

// ── Harness ──────────────────────────────────────────────────────────────────

#[contract]
pub struct GuardVersionHarness;

#[contractimpl]
impl GuardVersionHarness {
    pub fn init(env: Env, wrapped_contract: Address) {
        RuntimeGuardWrapper::init(env, wrapped_contract)
    }

    pub fn get_version(env: Env) -> u32 {
        RuntimeGuardWrapper::get_version(env)
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
}

// ── Fixtures ─────────────────────────────────────────────────────────────────

fn fixture_uninitialized(env: &Env) -> GuardVersionHarnessClient<'_> {
    let id = env.register_contract(None, GuardVersionHarness);
    GuardVersionHarnessClient::new(env, &id)
}

fn fixture_initialized(env: &Env) -> (GuardVersionHarnessClient<'_>, Address) {
    let client = fixture_uninitialized(env);
    let wrapped = Address::generate(env);
    client.init(&wrapped);
    (client, wrapped)
}

fn fixture_active(env: &Env) -> (GuardVersionHarnessClient<'_>, Address) {
    let (client, wrapped) = fixture_initialized(env);
    let empty = vec![env];
    let _ = client.execute_guarded(&Symbol::new(env, "ping"), &empty);
    let _ = client.execute_guarded(&Symbol::new(env, "echo"), &vec![env, 42u32.into_val(env)]);
    let _ = client.execute_guarded(
        &Symbol::new(env, "sum"),
        &vec![env, 3u32.into_val(env), 7u32.into_val(env)],
    );
    (client, wrapped)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[test]
fn version_constant_is_one() {
    assert_eq!(CONTRACT_VERSION, 1);
}

#[test]
fn get_version_returns_one_after_init() {
    let env = Env::default();
    let (client, _) = fixture_initialized(&env);
    assert_eq!(client.get_version(), 1);
}

#[test]
fn get_version_matches_contract_version_constant() {
    let env = Env::default();
    let (client, _) = fixture_initialized(&env);
    assert_eq!(client.get_version(), CONTRACT_VERSION);
}

#[test]
fn version_readable_immediately_after_init() {
    let env = Env::default();
    let client = fixture_uninitialized(&env);
    let wrapped = Address::generate(&env);

    client.init(&wrapped);

    assert_eq!(
        client.get_version(),
        1,
        "version must be readable right after init without any guarded calls"
    );
}

#[test]
fn reinit_does_not_change_version() {
    let env = Env::default();
    let (client, _) = fixture_initialized(&env);
    let replacement = Address::generate(&env);

    client.init(&replacement);

    assert_eq!(
        client.get_version(),
        1,
        "idempotent re-init must not alter the stored version"
    );
}

#[test]
fn version_persists_across_guarded_executions() {
    let env = Env::default();
    let (client, _) = fixture_active(&env);

    assert_eq!(
        client.get_version(),
        1,
        "normal guarded calls must not mutate the version key"
    );
}

#[test]
fn version_stable_with_high_call_volume() {
    let env = Env::default();
    let (client, _) = fixture_initialized(&env);

    for _ in 0..20u32 {
        let _ = client.execute_guarded(&Symbol::new(&env, "ping"), &vec![&env]);
    }

    assert_eq!(client.get_version(), 1);
}

#[test]
fn active_fixture_has_expected_stats_and_version() {
    let env = Env::default();
    let (client, _) = fixture_active(&env);

    let (inv_checked, call_count, failures) = client.get_stats();
    assert_eq!(inv_checked, 3);
    assert_eq!(call_count, 3);
    assert_eq!(failures, 0);
    assert_eq!(client.get_version(), 1);
}

#[test]
fn uninitialized_contract_falls_back_to_constant_version() {
    let env = Env::default();
    let client = fixture_uninitialized(&env);
    // No init called: version key absent, fallback must be CONTRACT_VERSION.
    assert_eq!(client.get_version(), CONTRACT_VERSION);
}
