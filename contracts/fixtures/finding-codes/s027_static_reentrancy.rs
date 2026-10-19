//! S027 — Static reentrancy (external call before state write).
//!
//! Demonstrates the checks-effects-interactions anti-pattern at the AST level:
//! an external call that precedes a storage mutation without a reentrancy guard.
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, vec, Address, Env};

#[contract]
pub struct StaticReentrancyBuggy;

#[contract]
pub struct StaticReentrancyGood;

#[contractimpl]
impl StaticReentrancyBuggy {
    // ❌ BAD (high confidence): invoke_contract (panicking) before storage write.
    pub fn unsafe_withdraw(env: Env, other: Address, amount: i128) {
        // Interaction first — callee can re-enter with stale balance
        let _result = env.invoke_contract::<()>(&other, &symbol_short!("recv"), vec![&env]);
        // Effect too late — reentrancy has already exploited stale state
        env.storage()
            .persistent()
            .set(&symbol_short!("BAL"), &amount);
    }

    // ❌ BAD (medium confidence): try_invoke_contract before storage write.
    pub fn try_call_then_write(env: Env, other: Address) {
        let _ = env.try_invoke_contract::<(), ()>(&other, &symbol_short!("cb"), vec![&env]);
        env.storage()
            .persistent()
            .set(&symbol_short!("STATE"), &1u32);
    }
}

#[contractimpl]
impl StaticReentrancyGood {
    // ✅ GOOD: effects first (CEI pattern), then interaction.
    pub fn safe_withdraw(env: Env, other: Address, amount: i128) {
        // Effect first
        env.storage()
            .persistent()
            .set(&symbol_short!("BAL"), &amount);
        // Interaction last — safe CEI order
        let _result = env.invoke_contract::<()>(&other, &symbol_short!("recv"), vec![&env]);
    }

    // ✅ GOOD: reentrancy guard around the external call.
    pub fn guarded_call(env: Env, other: Address) {
        let locked: bool = env
            .storage()
            .instance()
            .get(&symbol_short!("RE_LOCK"))
            .unwrap_or(false);
        if locked {
            panic!("reentrant call");
        }
        env.storage()
            .instance()
            .set(&symbol_short!("RE_LOCK"), &true);
        let _result = env.invoke_contract::<()>(&other, &symbol_short!("cb"), vec![&env]);
        env.storage()
            .persistent()
            .set(&symbol_short!("STATE"), &1u32);
        env.storage()
            .instance()
            .set(&symbol_short!("RE_LOCK"), &false);
    }
}
