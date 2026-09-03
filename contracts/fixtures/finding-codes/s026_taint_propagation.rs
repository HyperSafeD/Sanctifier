//! S026 — Taint propagation through tuple/struct destructures.
//!
//! Demonstrates cases where user-controlled data flows through destructuring
//! assignments to storage sinks without authentication.
#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct TaintBuggy;

#[contract]
pub struct TaintGood;

/// Helper struct for the struct-destructure test case.
pub struct KeyValuePair {
    pub key: Symbol,
    pub value: i128,
}

#[contractimpl]
impl TaintBuggy {
    // ❌ BAD: taint flows through tuple destructure to storage key.
    pub fn store_pair(env: Env, pair: (Symbol, i128)) {
        let (key, val) = pair;
        env.storage().persistent().set(&key, &val);
    }

    // ❌ BAD: taint flows through struct destructure to storage key.
    pub fn store_record(env: Env, record: KeyValuePair) {
        let KeyValuePair { key, value } = record;
        env.storage().persistent().set(&key, &value);
    }

    // ❌ BAD: direct parameter taint to storage — simpler but same issue.
    pub fn bad_set(env: Env, key: Symbol, val: i128) {
        env.storage().persistent().set(&key, &val);
    }
}

#[contractimpl]
impl TaintGood {
    // ✅ GOOD: require_auth gates the operation before any storage write.
    pub fn store_pair_safe(env: Env, caller: Address, pair: (Symbol, i128)) {
        caller.require_auth();
        let (key, val) = pair;
        env.storage().persistent().set(&key, &val);
    }
}
