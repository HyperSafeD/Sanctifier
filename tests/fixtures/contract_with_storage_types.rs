#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

// ── Storage layout ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    Counter,
    User(Address),
}

#[contracttype]
pub struct Config {
    pub max_value: u64,
    pub owner: Address,
}

// ── Contract ───────────────────────────────────────────────────────────────────

#[contract]
pub struct Registry;

#[contractimpl]
impl Registry {
    /// Reserved constructor — sets up initial state.
    pub fn __constructor(env: Env, admin: Address, max_value: u64) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Counter, &0u64);
        env.storage().instance().set(
            &DataKey::Admin,
            &Config {
                max_value,
                owner: admin,
            },
        );
    }

    pub fn increment(env: Env, caller: Address) -> u64 {
        caller.require_auth();
        let count: u64 = env.storage().instance().get(&DataKey::Counter).unwrap_or(0);
        let new_count = count.checked_add(1).expect("counter overflow");
        env.storage().instance().set(&DataKey::Counter, &new_count);
        new_count
    }

    pub fn get_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::Counter).unwrap_or(0)
    }

    pub fn admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }
}
