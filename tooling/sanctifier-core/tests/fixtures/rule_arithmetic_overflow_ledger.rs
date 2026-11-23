//! Fixture for the `arithmetic_overflow` rule: a reward ledger that adds to a
//! balance with a bare `+`. The caller is authenticated and an event is
//! emitted, so `arithmetic_overflow` is the only rule that should fire.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct RewardLedger;

#[contractimpl]
impl RewardLedger {
    pub fn credit(env: Env, user: Address, points: i128) -> i128 {
        user.require_auth();
        let key = symbol_short!("points");
        let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        let updated = current + points;
        env.storage().persistent().set(&key, &updated);
        env.storage().persistent().extend_ttl(&key, 100, 200);
        env.events().publish((symbol_short!("credit"), user), points);
        updated
    }

    pub fn balance(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&symbol_short!("points"))
            .unwrap_or(0)
    }
}
