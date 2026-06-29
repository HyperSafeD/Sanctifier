#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct AuthGapContract;

#[contractimpl]
impl AuthGapContract {
    pub fn store_user(env: Env, user: Address) {
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "user"), &user);
    }

    pub fn has_user(env: Env) -> bool {
        env.storage().instance().has(&Symbol::new(&env, "user"))
    }
}
