#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Address};

#[contract]
pub struct RequireAuthForArgsFixture;

#[contractimpl]
impl RequireAuthForArgsFixture {
    /// VULNERABLE: Multi-arg admin function using require_auth instead of require_auth_for_args
    /// This allows replay attacks where a signature for one (caller, new_admin) pair
    /// can be replayed with different new_admin values.
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth();
        env.storage().instance().set(&symbol_short!("admin"), &new_admin);
    }

    /// VULNERABLE: Three Address parameters with require_auth
    /// Attacker can replay a transfer_from signature with different from/to addresses
    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        env.storage().instance().set(&symbol_short!("balance"), &amount);
    }

    /// SAFE: Uses require_auth_for_args to bind auth to exact arguments
    pub fn set_admin_safe(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth_for_args((new_admin.clone(),).into_val(&env));
        env.storage().instance().set(&symbol_short!("admin"), &new_admin);
    }

    /// SAFE: Single Address parameter - require_auth is appropriate
    pub fn set_owner(env: Env, owner: Address) {
        owner.require_auth();
        env.storage().instance().set(&symbol_short!("owner"), &owner);
    }

    /// SAFE: Read-only function with multiple Address params - no auth needed
    pub fn check_permission(_env: Env, _user: Address, _admin: Address) -> bool {
        true
    }
}
