#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

#[contract]
pub struct VulnerableContract;

#[contractimpl]
impl VulnerableContract {
    // ❌ SECURITY FLAW: Missing authentication!
    // Anyone can call this and overwrite the admin.
    pub fn set_admin(env: Env, new_admin: Symbol) {
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &new_admin);
    }

    // ✅ Secure version
    pub fn set_admin_secure(env: Env, new_admin: Symbol) {
        let admin: Symbol = env
            .storage()
            .instance()
            .get(&symbol_short!("admin"))
            .expect("Admin not set");
        // env.require_auth(&admin); // Assume we can verify this if it were an Address
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &new_admin);
    }

    pub fn fail_explicitly(_env: Env) {
        panic!("Something went wrong");
    }
}
