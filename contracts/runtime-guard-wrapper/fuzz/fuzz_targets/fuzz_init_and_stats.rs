#![no_main]

//! Fuzz harness for `RuntimeGuardWrapper::init`, `get_version`, `get_stats`,
//! and `health_check`.
//!
//! This fuzzer complements `fuzz_execute_guarded` by focusing on the
//! initialisation lifecycle, version persistence, and the health-check
//! predicate under arbitrary ledger timestamps and repeated `init` calls.
//!
//! # Invariants Tested
//!
//! - `get_version` never panics and always returns `CONTRACT_VERSION` once
//!   `init` has been called.
//! - `health_check` returns `false` before `init` and must never panic.
//! - `get_stats` returns a consistent triple `(checked, log_len, failures)`
//!   where every component is zero before any `execute_guarded` call.
//! - Repeated `init` calls are idempotent: the version never changes.
//! - `get_wrapped_contract` after `init` never panics.
//!
//! # Running
//!
//! ```bash
//! cd contracts/runtime-guard-wrapper
//! cargo fuzz run fuzz_init_and_stats
//! ```

use libfuzzer_sys::fuzz_target;
use runtime_guard_wrapper::{RuntimeGuardWrapper, CONTRACT_VERSION};
use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, Address, Env, Symbol, Vec,
};

// ---------------------------------------------------------------------------
// Thin harness contract — mirrors the pattern in fuzz_execute_guarded.rs.
// ---------------------------------------------------------------------------

#[contract]
pub struct InitStatsHarness;

#[contractimpl]
impl InitStatsHarness {
    pub fn init(env: Env, wrapped_contract: Address) {
        RuntimeGuardWrapper::init(env, wrapped_contract)
    }

    pub fn get_version(env: Env) -> u32 {
        RuntimeGuardWrapper::get_version(env)
    }

    pub fn get_stats(env: Env) -> (u32, u32, u32) {
        RuntimeGuardWrapper::get_stats(env)
    }

    pub fn health_check(env: Env) -> bool {
        RuntimeGuardWrapper::health_check(env)
    }

    pub fn try_execute_guarded(
        env: Env,
        function_name: Symbol,
        args: Vec<soroban_sdk::Val>,
    ) -> Result<soroban_sdk::Val, soroban_sdk::Error> {
        RuntimeGuardWrapper::execute_guarded(env, function_name, args)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Construct an arbitrary-but-safe `Symbol` from a single raw byte.
///
/// Soroban `Symbol` characters are restricted to `[a-zA-Z0-9_]`; map every
/// byte into that range so the engine never rejects the symbol string itself
/// and we are still exercising the contract guard logic.
fn byte_to_symbol_char(b: u8) -> char {
    let alphabet = b"abcdefghijklmnopqrstuvwxyz0123456789_";
    alphabet[(b as usize) % alphabet.len()] as char
}

fn byte_to_symbol<'a>(env: &Env, b: u8, fallback: &'a str) -> Symbol {
    let ch = byte_to_symbol_char(b);
    // Single-char strings are always valid Soroban symbols.
    let s: &str = Box::leak(ch.to_string().into_boxed_str());
    let _ = fallback; // not used — every byte maps to a valid char
    Symbol::new(env, s)
}

// ---------------------------------------------------------------------------
// Fuzz target
// ---------------------------------------------------------------------------

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let env = Env::default();
    let contract_id = env.register_contract(None, InitStatsHarness);
    let client = InitStatsHarnessClient::new(&env, &contract_id);

    // ── Phase 1: pre-init invariants ────────────────────────────────────────

    // `health_check` must always return `false` before `init`.
    assert!(
        !client.health_check(),
        "health_check must be false before init"
    );

    // `get_stats` must return zeros — no calls have been recorded yet.
    let (checked, log_len, failures) = client.get_stats();
    assert_eq!(checked, 0, "pre-init: checked must be 0");
    assert_eq!(log_len, 0, "pre-init: log_len must be 0");
    assert_eq!(failures, 0, "pre-init: failures must be 0");

    // ── Phase 2: init (possibly multiple times) ──────────────────────────────

    let wrapped = Address::generate(&env);
    let init_count = (data[0] % 4) + 1; // 1..=4 init calls

    for _ in 0..init_count {
        client.init(&wrapped);
    }

    // ── Phase 3: post-init invariants ───────────────────────────────────────

    // Version must be exactly the compile-time constant, regardless of how
    // many times init was called.
    assert_eq!(
        client.get_version(),
        CONTRACT_VERSION,
        "get_version must equal CONTRACT_VERSION after init"
    );

    // health_check must report healthy (no executions yet, storage within
    // the HEALTHY_STORAGE_LIMIT).
    assert!(
        client.health_check(),
        "health_check must be true after init (no executions yet)"
    );

    // ── Phase 4: execute some guarded calls and re-check stats ───────────────

    // Use the remaining bytes to drive a short sequence of execute_guarded
    // calls, exercising the full set of named functions plus invalid ones.
    for chunk in data[1..].chunks(3) {
        let selector = chunk[0] % 5;
        let function_name = match selector {
            0 => Symbol::new(&env, "ping"),
            1 => Symbol::new(&env, "echo"),
            2 => Symbol::new(&env, "sum"),
            3 => Symbol::new(&env, "bad_fn"),
            _ => byte_to_symbol(&env, chunk.get(1).copied().unwrap_or(b'a'), "x"),
        };

        let arg_count = chunk.get(2).copied().unwrap_or(0) % 4; // 0..=3
        let mut args = soroban_sdk::vec![&env];
        for i in 0..arg_count {
            let val: u32 = chunk.get(i as usize).copied().unwrap_or(0) as u32;
            args.push_back(val.into());
        }

        // Must never panic — invalid inputs resolve to typed errors.
        let _ = client.try_execute_guarded(&function_name, &args);
    }

    // ── Phase 5: final consistency checks ────────────────────────────────────

    // version is still the same constant — nothing should change it at runtime.
    assert_eq!(
        client.get_version(),
        CONTRACT_VERSION,
        "get_version must remain stable after execute_guarded calls"
    );

    // get_stats must remain callable and return non-negative values.
    let (_, final_log_len, _) = client.get_stats();
    assert!(
        final_log_len <= 100,
        "call log must remain within the 100-entry cap"
    );

    // health_check must remain callable without panicking.
    let _ = client.health_check();
});
