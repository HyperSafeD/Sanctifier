#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct MissingEventFixture;

#[contractimpl]
impl MissingEventFixture {
    pub fn set_admin(env: Env, new_admin: Address) {
        let admin: Address = env.storage().instance().get(&symbol_short!("ADMIN")).unwrap();
        admin.require_auth();
        
        // ❌ RISK: Critical state change (admin update) WITHOUT event emission.
        env.storage().instance().set(&symbol_short!("ADMIN"), &new_admin);
    }
}
