// Example Rust source file that triggers CUSTOM-AUTH-001 from custom-vulndb.json
// This file demonstrates a missing authentication check in an admin function

use soroban_sdk::{contract, contractimpl, Env, Storage, Address};

#[contract]
pub struct AdminContract;

#[contractimpl]
impl AdminContract {
    // VULNERABILITY: This admin function modifies storage without authentication
    #[pub]
    fn admin_update_config(env: Env, new_value: u32) {
        // Missing: let admin = env.invoker(); admin.require_auth();
        let mut storage = env.storage().persistent();
        storage.set(&"config", &new_value);
    }

    // SECURE VERSION (for comparison):
    #[pub]
    fn secure_admin_update(env: Env, admin: Address, new_value: u32) {
        admin.require_auth(); // Proper authentication check
        let mut storage = env.storage().persistent();
        storage.set(&"config", &new_value);
    }
}
