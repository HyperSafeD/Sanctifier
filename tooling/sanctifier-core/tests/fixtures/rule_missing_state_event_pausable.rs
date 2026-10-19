//! Fixture for the `missing_state_event` rule: a pausable contract whose
//! `pause` flips a privileged flag without publishing an event, so indexers
//! and monitors never see it. The caller is authenticated and the storage
//! entry's TTL is extended. `shadow_storage` reports the same missing event
//! from a broader angle, so it is the one companion finding expected here.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct Pausable;

#[contractimpl]
impl Pausable {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&symbol_short!("admin"), &admin);
        env.events().publish((symbol_short!("init"),), admin);
    }

    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().persistent().set(&symbol_short!("paused"), &true);
        env.storage()
            .persistent()
            .extend_ttl(&symbol_short!("paused"), 100, 200);
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .persistent()
            .get(&symbol_short!("paused"))
            .unwrap_or(false)
    }
}
