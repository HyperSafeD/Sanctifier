#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env, Symbol};

/// Typed payload for the `admin_set` event (issue #1445), matching the
/// `contracttype` + `events().publish((topic,), data)` convention already
/// used elsewhere in this workspace (e.g. `contracts/flashloan-token`),
/// rather than publishing loose tuples an indexer would have to guess the
/// shape of.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminSetEvent {
    pub new_admin: Symbol,
}

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
        env.events().publish(
            (symbol_short!("admin_set"),),
            AdminSetEvent {
                new_admin: new_admin.clone(),
            },
        );
    }

    // ✅ Secure version
    pub fn set_admin_secure(env: Env, new_admin: Symbol) {
        let _admin: Symbol = env
            .storage()
            .instance()
            .get(&symbol_short!("admin"))
            .expect("Admin not set");
        // env.require_auth(&admin); // Assume we can verify this if it were an Address
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &new_admin);
        env.events().publish(
            (symbol_short!("admin_set"),),
            AdminSetEvent {
                new_admin: new_admin.clone(),
            },
        );
    }

    pub fn fail_explicitly(_env: Env) {
        panic!("Something went wrong");
    }
}
