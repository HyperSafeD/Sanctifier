#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Env, Symbol};

/// Storage keys for this contract. A `contracttype` enum instead of raw
/// `Symbol`s (issue #1466) so every storage slot is a distinct, compiler-
/// checked variant -- a typo'd string key silently reading/writing the wrong
/// slot is no longer possible, and adding a new key can't accidentally
/// collide with an existing one.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
}

#[contract]
pub struct VulnerableContract;

#[contractimpl]
impl VulnerableContract {
    // ❌ SECURITY FLAW: Missing authentication!
    // Anyone can call this and overwrite the admin.
    pub fn set_admin(env: Env, new_admin: Symbol) {
        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }

    // ✅ Secure version
    pub fn set_admin_secure(env: Env, new_admin: Symbol) {
        let _admin: Symbol = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        // env.require_auth(&admin); // Assume we can verify this if it were an Address
        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }

    pub fn fail_explicitly(_env: Env) {
        panic!("Something went wrong");
    }
}
