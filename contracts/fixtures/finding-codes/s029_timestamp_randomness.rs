//! S029 — Timestamp used as randomness entropy.
//!
//! Block timestamps are not secret entropy. Validators can nudge
//! `env.ledger().timestamp()` within a window, making any randomness
//! derived solely from it manipulable.
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Vec};

#[contract]
pub struct TimestampRandomnessBuggy;

#[contract]
pub struct TimestampRandomnessSafe;

#[contractimpl]
impl TimestampRandomnessBuggy {
    // ❌ BAD: timestamp used as seed — flagged because function name contains "seed".
    pub fn seed_prng(env: Env) -> u64 {
        let seed = env.ledger().timestamp();
        seed % 1000
    }

    // ❌ BAD: timestamp used as randomness — function name contains "rand".
    pub fn rand_roll(env: Env) -> u64 {
        env.ledger().timestamp() % 6 + 1
    }

    // ❌ BAD: timestamp selects winner — function name contains "pick".
    pub fn pick_winner(env: Env, participants: Vec<Address>) -> Address {
        let idx = env.ledger().timestamp() % participants.len() as u64;
        participants.get(idx as u32).unwrap()
    }

    // ❌ BAD: timestamp stored into a variable named "winner".
    pub fn draw(env: Env, total: u64) -> u64 {
        let winner = env.ledger().timestamp() % total;
        winner
    }
}

#[contractimpl]
impl TimestampRandomnessSafe {
    // ✅ SAFE: timestamp used for expiry/time comparison only.
    pub fn check_expiry(env: Env, deadline: u64) -> bool {
        env.ledger().timestamp() > deadline
    }

    // ✅ SAFE: timestamp used for logging/record keeping, not as entropy.
    pub fn record_time(env: Env) {
        let ts = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&symbol_short!("LAST_TS"), &ts);
    }
}
