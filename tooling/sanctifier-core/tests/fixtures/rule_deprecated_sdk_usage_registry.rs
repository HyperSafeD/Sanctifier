//! Fixture for the `deprecated_sdk_usage` rule: a registry written against
//! Soroban SDK v21 that calls `storage().persistent().bump(..)`. The caller is
//! authenticated and an event is emitted, so `deprecated_sdk_usage` is the
//! only rule that should fire.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct Registry;

#[contractimpl]
impl Registry {
    pub fn register(env: Env, owner: Address, id: u32) {
        owner.require_auth();
        let key = symbol_short!("entry");
        env.storage().persistent().set(&key, &id);
        env.storage().persistent().bump(&key, 100_000);
        env.events().publish((symbol_short!("register"), owner), id);
    }

    pub fn lookup(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&symbol_short!("entry"))
            .unwrap_or(0)
    }
}
