#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct StorageMisuseFixture;

#[contractimpl]
impl StorageMisuseFixture {
    pub fn set_user_data(env: Env, user: Address, data: soroban_sdk::String) {
        // ❌ RISK: Per-user data stored in Instance storage.
        // Instance storage is limited; use Persistent for user-specific data.
        env.storage().instance().set(&user, &data);
    }
}
