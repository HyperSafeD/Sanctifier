#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, symbol_short, Address, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AdminError {
    /// `init` has not been called, so there is no admin to authorise against.
    AdminNotSet = 1,
}

#[contract]
pub struct AdminTrustFixture;

#[contractimpl]
impl AdminTrustFixture {
    pub fn init(env: Env, admin: Address) {
        env.storage().instance().set(&symbol_short!("ADMIN"), &admin);
    }

    // ❌ RISK: Admin has absolute power to wipe any user's balance without cause.
    // While technically functional, this centralisation risk should be flagged.
    pub fn admin_wipe(env: Env, user: Address) -> Result<(), AdminError> {
        // A missing admin is reported as an error rather than trapping the
        // contract with an unrecoverable `unwrap()` panic.
        let admin: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("ADMIN"))
            .ok_or(AdminError::AdminNotSet)?;
        admin.require_auth();
        env.storage().persistent().set(&symbol_short!("BAL"), &0i128);
        Ok(())
    }
}
