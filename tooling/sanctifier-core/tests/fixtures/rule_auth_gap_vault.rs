//! Fixture for the `auth_gap` rule: a vault whose `reset_limit` writes contract
//! storage without authenticating its caller. Every other function is
//! authenticated, uses checked arithmetic and emits an event, so `auth_gap`
//! is the only rule that should fire.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct Vault;

#[contractimpl]
impl Vault {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&symbol_short!("admin"), &admin);
        env.events().publish((symbol_short!("init"),), admin);
    }

    pub fn deposit(env: Env, from: Address, amount: i128) {
        from.require_auth();
        let key = symbol_short!("total");
        let total: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        let updated = total.checked_add(amount).unwrap_or(total);
        env.storage().persistent().set(&key, &updated);
        env.storage().persistent().extend_ttl(&key, 100, 200);
        env.events().publish((symbol_short!("deposit"), from), amount);
    }

    pub fn reset_limit(env: Env) {
        let key = symbol_short!("limit");
        env.storage().persistent().set(&key, &0i128);
        env.storage().persistent().extend_ttl(&key, 100, 200);
        env.events().publish((symbol_short!("reset"),), 0i128);
    }

    pub fn limit(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&symbol_short!("limit"))
            .unwrap_or(0)
    }
}
