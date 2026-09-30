#![no_main]

//! Fuzz test for `RuntimeGuardWrapper::execute_guarded` and its public
//! accessors.
//!
//! This fuzzer drives the contract through a `soroban_sdk` test `Env`,
//! replaying a sequence of `execute_guarded` calls built from arbitrary
//! bytes — random function names, random argument counts and values, with
//! or without a prior `init`. The contract must never panic: every
//! unexpected input must resolve to a typed `RuntimeGuardError` returned
//! through `Result`, not a trap.
//!
//! # Invariants Tested
//!
//! - `health_check` is `false` before `init` and never panics.
//! - `execute_guarded` never panics regardless of function name or arity.
//! - The call log never grows past its 100-entry cap (#lib.rs `log_execution`).
//! - `get_stats` and `get_version` stay callable after any sequence of calls.
//!
//! # Running
//!
//! ```bash
//! cd contracts/runtime-guard-wrapper
//! cargo fuzz run fuzz_execute_guarded
//! ```

use libfuzzer_sys::fuzz_target;
use runtime_guard_wrapper::RuntimeGuardWrapper;
use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, vec, Address, Env, IntoVal, Symbol, Val, Vec,
};

// Thin re-export contract so the fuzz crate can drive `RuntimeGuardWrapper`
// through a real `Env`/`Client`, mirroring the harness pattern used in
// `tests/integration_tests.rs`.
#[contract]
pub struct FuzzHarness;

#[contractimpl]
impl FuzzHarness {
    pub fn init(env: Env, wrapped_contract: Address) {
        RuntimeGuardWrapper::init(env, wrapped_contract)
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

    pub fn get_version(env: Env) -> u32 {
        RuntimeGuardWrapper::get_version(env)
    }

    pub fn health_check(env: Env) -> bool {
        RuntimeGuardWrapper::health_check(env)
    }
}

/// Map an arbitrary byte to a lowercase ASCII letter so it forms a valid
/// Soroban `Symbol` character (`[a-zA-Z0-9_]`).
fn safe_symbol_char(byte: u8) -> char {
    (b'a' + (byte % 26)) as char
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    let env = Env::default();
    let contract_id = env.register_contract(None, FuzzHarness);
    let wrapped = Address::generate(&env);
    let client = FuzzHarnessClient::new(&env, &contract_id);

    let initialized = data[0] % 2 == 0;
    if initialized {
        client.init(&wrapped);
        // Re-init is documented as idempotent (#1415) — exercise it too.
        if data[1] % 4 == 0 {
            client.init(&wrapped);
        }
    } else {
        assert!(
            !client.health_check(),
            "health_check must report unhealthy before init"
        );
    }

    // Replay a sequence of guarded calls, six bytes per operation:
    // [selector, arg_count, name_byte0, name_byte1, arg0, arg1]
    for chunk in data[2..].chunks(6) {
        let selector = chunk[0];
        let function_name = match selector % 5 {
            0 => Symbol::new(&env, "ping"),
            1 => Symbol::new(&env, "echo"),
            2 => Symbol::new(&env, "sum"),
            3 => Symbol::new(&env, "bad_fn"),
            _ => {
                let name: String = chunk
                    .get(2..4)
                    .unwrap_or(&[])
                    .iter()
                    .map(|b| safe_symbol_char(*b))
                    .collect();
                Symbol::new(&env, if name.is_empty() { "x" } else { &name })
            }
        };

        let arg_count = chunk.get(1).copied().unwrap_or(0) % 3;
        let mut args = vec![&env];
        for i in 0..arg_count {
            let val = chunk.get(4 + i as usize).copied().unwrap_or(0) as u32;
            args.push_back(val.into_val(&env));
        }

        // Must never panic — invalid names/arities resolve to `Err`.
        let _ = client.try_execute_guarded(&function_name, &args);
    }

    // Whatever happened above, the public accessors must stay callable and
    // internally consistent.
    let (checked, log_len, failures) = client.get_stats();
    assert!(
        log_len <= 100,
        "call log must stay within its documented 100-entry cap"
    );
    let _ = checked;
    let _ = failures;
    let _ = client.health_check();
    let _ = client.get_version();
});
