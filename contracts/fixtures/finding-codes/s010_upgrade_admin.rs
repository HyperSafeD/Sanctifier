#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, symbol_short, Address, BytesN, Env, Symbol};

const ADMIN: Symbol = symbol_short!("ADMIN");
const IMPL_HASH: Symbol = symbol_short!("IMPLHASH");

#[contracterror]
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum UpgradeAdminError {
    NotInitialized = 1,
    Unauthorized = 2,
    SameImplementationHash = 3,
    SameAdmin = 4,
}

#[contract]
pub struct UpgradeAdminFixture;

#[contractimpl]
impl UpgradeAdminFixture {
    pub fn initialize(env: Env, admin: Address, impl_hash: BytesN<32>) {
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&IMPL_HASH, &impl_hash);
    }

    pub fn insecure_upgrade(env: Env, new_hash: BytesN<32>) {
        env.storage().instance().set(&IMPL_HASH, &new_hash);
    }

    pub fn secure_upgrade(env: Env, caller: Address, new_hash: BytesN<32>) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN)
            .unwrap_or_else(|| env.panic_with_error(UpgradeAdminError::NotInitialized));

        caller.require_auth();
        if caller != admin {
            env.panic_with_error(UpgradeAdminError::Unauthorized);
        }

        let current: BytesN<32> = env
            .storage()
            .instance()
            .get(&IMPL_HASH)
            .unwrap_or_else(|| env.panic_with_error(UpgradeAdminError::NotInitialized));
        if current == new_hash {
            env.panic_with_error(UpgradeAdminError::SameImplementationHash);
        }

        env.storage().instance().set(&IMPL_HASH, &new_hash);
    }

    pub fn insecure_transfer_admin(env: Env, new_admin: Address) {
        env.storage().instance().set(&ADMIN, &new_admin);
    }

    pub fn secure_transfer_admin(env: Env, caller: Address, new_admin: Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN)
            .unwrap_or_else(|| env.panic_with_error(UpgradeAdminError::NotInitialized));

        caller.require_auth();
        if caller != admin {
            env.panic_with_error(UpgradeAdminError::Unauthorized);
        }
        if new_admin == admin {
            env.panic_with_error(UpgradeAdminError::SameAdmin);
        }

        env.storage().instance().set(&ADMIN, &new_admin);
    }
}
