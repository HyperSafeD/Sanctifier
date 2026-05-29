#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct AdminTrustFixture;

#[contractimpl]
impl AdminTrustFixture {
    pub fn init(env: Env, admin: Address) {
        env.storage().instance().set(&symbol_short!("ADMIN"), &admin);
    }

    // ❌ RISK: Admin has absolute power to wipe any user's balance without cause.
    // While technically functional, this centralisation risk should be flagged.
    pub fn admin_wipe(env: Env, user: Address) {
        let admin: Address = env.storage().instance().get(&symbol_short!("ADMIN")).unwrap();
        admin.require_auth();
        env.storage().persistent().set(&symbol_short!("BAL"), &0i128);
    }
}
