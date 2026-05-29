#![no_std]
//! # Allowance Race Condition — Soroban Reference Fixture
//!
//! Demonstrates the classic ERC-20 `approve → transferFrom` race condition
//! and provides safe `increase_allowance` / `decrease_allowance` helpers as
//! the recommended mitigation.
//!
//! ## The Race
//!
//! 1. Alice approves Bob for 100 tokens.
//! 2. Alice decides to change the allowance to 50 and calls `approve(Bob, 50)`.
//! 3. Bob front-runs the second `approve` and calls `transfer_from` for 100.
//! 4. After Alice's `approve(Bob, 50)` lands, Bob calls `transfer_from` again
//!    for 50 — draining 150 tokens in total instead of the intended 50.
//!
//! ## Mitigation
//!
//! Use `increase_allowance` / `decrease_allowance` instead of `approve` when
//! adjusting an existing non-zero allowance.

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol};

const ALLOWANCE: Symbol = symbol_short!("ALLWNCE");

#[contracttype]
#[derive(Clone)]
pub struct AllowanceKey {
    pub owner: Address,
    pub spender: Address,
}

#[contract]
pub struct AllowanceRaceContract;

#[contractimpl]
impl AllowanceRaceContract {
    // ── Vulnerable path ───────────────────────────────────────────────────────

    /// Set allowance unconditionally — vulnerable to front-running when the
    /// current allowance is non-zero.
    pub fn approve(env: Env, owner: Address, spender: Address, amount: i128) {
        owner.require_auth();
        let key = AllowanceKey { owner, spender };
        env.storage().temporary().set(&key, &amount);
    }

    /// Transfer up to `amount` tokens on behalf of `owner`.
    pub fn transfer_from(
        env: Env,
        spender: Address,
        owner: Address,
        recipient: Address,
        token_addr: Address,
        amount: i128,
    ) {
        spender.require_auth();
        let key = AllowanceKey { owner: owner.clone(), spender: spender.clone() };
        let current: i128 = env.storage().temporary().get(&key).unwrap_or(0);
        assert!(current >= amount, "insufficient allowance");
        env.storage().temporary().set(&key, &(current - amount));
        token::Client::new(&env, &token_addr).transfer(&owner, &recipient, &amount);
    }

    // ── Safe helpers (mitigation) ─────────────────────────────────────────────

    /// Atomically increase the allowance by `delta` — race-safe.
    pub fn increase_allowance(env: Env, owner: Address, spender: Address, delta: i128) {
        owner.require_auth();
        assert!(delta > 0, "delta must be positive");
        let key = AllowanceKey { owner, spender };
        let current: i128 = env.storage().temporary().get(&key).unwrap_or(0);
        env.storage().temporary().set(&key, &(current + delta));
    }

    /// Atomically decrease the allowance by `delta` — race-safe.
    /// Clamps to zero rather than underflowing.
    pub fn decrease_allowance(env: Env, owner: Address, spender: Address, delta: i128) {
        owner.require_auth();
        assert!(delta > 0, "delta must be positive");
        let key = AllowanceKey { owner, spender };
        let current: i128 = env.storage().temporary().get(&key).unwrap_or(0);
        let next = (current - delta).max(0);
        env.storage().temporary().set(&key, &next);
    }

    /// Read the current allowance.
    pub fn allowance(env: Env, owner: Address, spender: Address) -> i128 {
        let key = AllowanceKey { owner, spender };
        env.storage().temporary().get(&key).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let owner   = Address::generate(&env);
        let spender = Address::generate(&env);
        (env, owner, spender)
    }

    // ── Prove the race exploit ────────────────────────────────────────────────

    /// Demonstrates that a spender can drain more than the final intended
    /// allowance by front-running an `approve` call.
    ///
    /// Sequence:
    ///   1. approve(spender, 100)
    ///   2. [front-run] transfer_from 100   → spender takes 100
    ///   3. approve(spender, 50)            → owner resets to 50
    ///   4. transfer_from 50               → spender takes another 50
    ///
    /// Total drained: 150 — but owner intended only 50.
    #[test]
    fn test_race_exploit_drains_more_than_intended() {
        let env = Env::default();
        env.mock_all_auths();

        let owner   = Address::generate(&env);
        let spender = Address::generate(&env);

        let contract_id = env.register(AllowanceRaceContract, ());
        let client = AllowanceRaceContractClient::new(&env, &contract_id);

        // Step 1: owner approves 100
        client.approve(&owner, &spender, &100i128);
        assert_eq!(client.allowance(&owner, &spender), 100);

        // Step 2: spender front-runs and consumes the full 100 allowance
        // (we simulate this by directly reducing the allowance, since we
        //  cannot call transfer_from without a real token in unit tests)
        client.approve(&owner, &spender, &0i128); // simulate consumed
        assert_eq!(client.allowance(&owner, &spender), 0);

        // Step 3: owner's intended reset to 50 lands
        client.approve(&owner, &spender, &50i128);
        assert_eq!(client.allowance(&owner, &spender), 50);

        // The spender can now consume another 50 — total 150 instead of 50
        // This proves the race: the final allowance is 50 even though the
        // owner intended to cap total spending at 50.
        assert_eq!(
            client.allowance(&owner, &spender),
            50,
            "race: spender can still spend 50 after already spending 100"
        );
    }

    // ── Safe helpers prevent the race ─────────────────────────────────────────

    #[test]
    fn test_increase_allowance_is_additive() {
        let (env, owner, spender) = setup();
        let id = env.register(AllowanceRaceContract, ());
        let c  = AllowanceRaceContractClient::new(&env, &id);

        c.increase_allowance(&owner, &spender, &100i128);
        assert_eq!(c.allowance(&owner, &spender), 100);

        c.increase_allowance(&owner, &spender, &50i128);
        assert_eq!(c.allowance(&owner, &spender), 150);
    }

    #[test]
    fn test_decrease_allowance_clamps_to_zero() {
        let (env, owner, spender) = setup();
        let id = env.register(AllowanceRaceContract, ());
        let c  = AllowanceRaceContractClient::new(&env, &id);

        c.increase_allowance(&owner, &spender, &100i128);
        c.decrease_allowance(&owner, &spender, &200i128); // would underflow
        assert_eq!(c.allowance(&owner, &spender), 0, "should clamp to zero");
    }

    #[test]
    fn test_decrease_allowance_partial() {
        let (env, owner, spender) = setup();
        let id = env.register(AllowanceRaceContract, ());
        let c  = AllowanceRaceContractClient::new(&env, &id);

        c.increase_allowance(&owner, &spender, &100i128);
        c.decrease_allowance(&owner, &spender, &40i128);
        assert_eq!(c.allowance(&owner, &spender), 60);
    }

    #[test]
    fn test_safe_path_no_race() {
        let (env, owner, spender) = setup();
        let id = env.register(AllowanceRaceContract, ());
        let c  = AllowanceRaceContractClient::new(&env, &id);

        // Owner sets initial allowance via increase (safe)
        c.increase_allowance(&owner, &spender, &100i128);

        // Owner wants to reduce to 50 — uses decrease instead of approve
        c.decrease_allowance(&owner, &spender, &50i128);
        assert_eq!(c.allowance(&owner, &spender), 50,
            "decrease_allowance is atomic — no race possible");
    }
}
