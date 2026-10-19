#![cfg(test)]
#![allow(unexpected_cfgs)]

use runtime_guard_wrapper::{
    RuntimeGuardWrapper, ERR_ARGUMENT_COUNT_MISMATCH, ERR_UNKNOWN_FUNCTION,
    ERR_WRAPPED_CONTRACT_NOT_SET,
};
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
    let wrapped = Address::generate(env);
    let client = RuntimeGuardWrapperHarnessClient::new(env, &contract_id);
    client.init(&wrapped);
    (client, wrapped)
}

// ── Error code assertions ──────────────────────────────────────────────────────

/// Unknown function returns ERR_UNKNOWN_FUNCTION (code 3).
#[test]
fn execute_guarded_rejects_missing_function_name() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let result = client.try_execute_guarded(&Symbol::new(&env, "missing"), &vec![&env]);

    assert!(result.is_err());
    let err = result.unwrap_err().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(ERR_UNKNOWN_FUNCTION)
    );
    assert_eq!(client.get_stats(), (0, 0, 0));
}

/// Wrong arity returns ERR_ARGUMENT_COUNT_MISMATCH (code 4).
#[test]
fn execute_guarded_rejects_argument_count_mismatch() {
    let env = Env::default();
    let (client, _) = setup(&env);
    // `ping` expects 0 args; pass 1.
    let args = vec![&env, 7u32.into_val(&env)];
    let result = client.try_execute_guarded(&Symbol::new(&env, "ping"), &args);

    assert!(result.is_err());
    let err = result.unwrap_err().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(ERR_ARGUMENT_COUNT_MISMATCH)
    );
    assert_eq!(client.get_stats(), (0, 0, 0));
}

// ── Idempotency ────────────────────────────────────────────────────────────────

#[test]
fn init_called_twice_is_idempotent() {
    let env = Env::default();
    let (client, wrapped) = setup(&env);
    let replacement = Address::generate(&env);

    client.init(&replacement);

    assert_eq!(client.get_wrapped_contract(), wrapped);
    assert_eq!(client.get_stats(), (0, 0, 0));
}

// ── Storage budget ─────────────────────────────────────────────────────────────

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

// ── Stats tracking ─────────────────────────────────────────────────────────────

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

#[test]
fn execute_guarded_tracks_wrapped_call_result_errors() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let invalid_sum_args = vec![
        &env,
        Symbol::new(&env, "badarg").into_val(&env),
        3u32.into_val(&env),
    ];

    let result = client.try_execute_guarded(&Symbol::new(&env, "sum"), &invalid_sum_args);

    assert!(result.is_err());
    assert_eq!(client.get_stats(), (0, 0, 0));
}

// ── Guard wrapper samples ──────────────────────────────────────────────────────
//
// The tests below demonstrate the complete guard lifecycle for each supported
// function.  They serve as living examples that contributors can copy when
// registering new wrapped functions.

/// Sample: `ping` — zero-argument round-trip through all three guard stages.
#[test]
fn guard_wrapper_sample_ping_round_trip() {
    let env = Env::default();
    let (client, _) = setup(&env);

    // pre-guard + execute + post-guard must all succeed for a valid call.
    let result = client.execute_guarded(&Symbol::new(&env, "ping"), &vec![&env]);
    let _ = result; // returns "pong" Symbol

    // invariants_checked incremented by post-execution guard.
    let (invariants_checked, call_log_len, failures) = client.get_stats();
    assert_eq!(
        invariants_checked, 1,
        "post-guard must increment invariants_checked"
    );
    assert_eq!(call_log_len, 1, "call must be logged");
    assert_eq!(failures, 0, "no failures expected");
}

/// Sample: `echo` — single-argument pass-through.
#[test]
fn guard_wrapper_sample_echo_pass_through() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let arg_val: Val = 42u32.into_val(&env);
    let _ = client.execute_guarded(&Symbol::new(&env, "echo"), &vec![&env, arg_val]);

    let (invariants_checked, call_log_len, failures) = client.get_stats();
    assert_eq!(invariants_checked, 1);
    assert_eq!(call_log_len, 1);
    assert_eq!(failures, 0);
}

/// Sample: `sum` — two-argument arithmetic with correct types.
#[test]
fn guard_wrapper_sample_sum_two_u32_args() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let result = client.execute_guarded(
        &Symbol::new(&env, "sum"),
        &vec![&env, 10u32.into_val(&env), 20u32.into_val(&env)],
    );
    let _ = result; // expected: 30u32 as Val

    let (invariants_checked, call_log_len, failures) = client.get_stats();
    assert_eq!(invariants_checked, 1);
    assert_eq!(call_log_len, 1);
    assert_eq!(failures, 0);
}

/// Sample: error path — confirms ERR_WRAPPED_CONTRACT_NOT_SET is returned when
/// the contract is accessed without `init`.
#[test]
fn guard_wrapper_sample_uninitialised_contract_returns_error() {
    let env = Env::default();
    // Register the harness contract but do NOT call init.
    let contract_id = env.register_contract(None, RuntimeGuardWrapperHarness);
    let client = RuntimeGuardWrapperHarnessClient::new(&env, &contract_id);

    let result = client.try_execute_guarded(&Symbol::new(&env, "ping"), &vec![&env]);

    assert!(result.is_err());
    let err = result.unwrap_err().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(ERR_WRAPPED_CONTRACT_NOT_SET)
    );
}

/// Sample: sequential calls accumulate stats correctly.
#[test]
fn guard_wrapper_sample_multiple_calls_accumulate_stats() {
    let env = Env::default();
    let (client, _) = setup(&env);

    for _ in 0..5 {
        let _ = client.execute_guarded(&Symbol::new(&env, "ping"), &vec![&env]);
    }

    let (invariants_checked, call_log_len, failures) = client.get_stats();
    assert_eq!(invariants_checked, 5);
    assert_eq!(call_log_len, 5);
    assert_eq!(failures, 0);
}
