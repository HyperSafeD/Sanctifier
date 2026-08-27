#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env, Symbol};

/// Storage keys using contracttype enum to prevent collisions.
///
/// Using an enum ensures:
/// - Type-safe key access
/// - No string-based key collisions
/// - Compiler-checked exhaustiveness
/// - Better refactoring support
#[contracttype]
#[derive(Clone, Copy)]
pub enum StorageKey {
    /// Contract administrator
    Admin,
    /// User balance storage (will be extended with user ID in practice)
    Balance,
    /// Contract configuration
    Config,
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
            .set(&StorageKey::Admin, &new_admin);
    }

    // ✅ Secure version with proper authentication
    pub fn set_admin_secure(env: Env, new_admin: Symbol) {
        let _admin: Symbol = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .expect("Admin not set");
        // env.require_auth(&admin); // Assume we can verify this if it were an Address
        env.storage()
            .instance()
            .set(&StorageKey::Admin, &new_admin);
    }

    /// Get current admin (demonstrates key reuse safety)
    pub fn get_admin(env: Env) -> Option<Symbol> {
        env.storage().instance().get(&StorageKey::Admin)
    }

    /// Initialize admin (demonstrates proper setup pattern)
    pub fn init_admin(env: Env, admin: Symbol) {
        // Check if already initialized to prevent re-initialization
        if env.storage().instance().has(&StorageKey::Admin) {
            panic!("Admin already initialized");
        }
        env.storage().instance().set(&StorageKey::Admin, &admin);
    }

    // ❌ SECURITY FLAW: Unhandled panic
    pub fn fail_explicitly(_env: Env) {
        panic!("Something went wrong");
    }

    // ✅ Improved version with Result return type
    pub fn fail_gracefully(_env: Env) -> Result<(), Symbol> {
        Err(symbol_short!("ERR"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Events, Env};

    #[test]
    fn test_storage_key_uniqueness() {
        let env = Env::default();
        let contract_id = env.register_contract(None, VulnerableContract);
        let client = VulnerableContractClient::new(&env, &contract_id);

        // Test that different keys don't collide
        let admin1 = symbol_short!("alice");
        client.init_admin(&admin1);

        let retrieved_admin = client.get_admin();
        assert_eq!(retrieved_admin, Some(admin1));
    }

    #[test]
    #[should_panic(expected = "Admin already initialized")]
    fn test_double_initialization_prevented() {
        let env = Env::default();
        let contract_id = env.register_contract(None, VulnerableContract);
        let client = VulnerableContractClient::new(&env, &contract_id);

        let admin1 = symbol_short!("alice");
        client.init_admin(&admin1);

        // Second initialization should fail
        let admin2 = symbol_short!("bob");
        client.init_admin(&admin2);
    }
}

