//! S025 — Missing TTL bump on Persistent/Temporary storage write.
//!
//! Demonstrates contracts that write to expiring storage tiers without calling
//! `extend_ttl`, causing silent data loss once the ledger TTL elapses.
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

#[contract]
pub struct MissingTtlBuggy;

#[contract]
pub struct MissingTtlGood;

#[contractimpl]
impl MissingTtlBuggy {
    // ❌ BAD: writes to persistent storage without bumping TTL.
    pub fn store(env: Env, key: Symbol, val: i128) {
        env.storage().persistent().set(&key, &val);
        // Missing: env.storage().persistent().extend_ttl(&key, 1000, 5000);
    }

    // ❌ BAD: temporary write also needs a TTL bump.
    pub fn cache(env: Env, key: Symbol, val: i128) {
        env.storage().temporary().set(&key, &val);
    }
}

#[contractimpl]
impl MissingTtlGood {
    // ✅ GOOD: write is accompanied by extend_ttl.
    pub fn store_safe(env: Env, key: Symbol, val: i128) {
        env.storage().persistent().set(&key, &val);
        env.storage().persistent().extend_ttl(&key, 1000, 5000);
    }

    // ✅ GOOD: instance storage has different lifecycle — not flagged.
    pub fn set_instance_flag(env: Env) {
        env.storage()
            .instance()
            .set(&symbol_short!("flag"), &true);
    }
}
