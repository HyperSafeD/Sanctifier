//! Fixture for the `instance_storage_misuse` rule: per-user balances written
//! to instance storage, which is one ledger entry shared by the whole contract
//! and grows with every user. The caller is authenticated and an event is
//! emitted, so `instance_storage_misuse` is the only rule that should fire.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct Points;

#[contractimpl]
impl Points {
    pub fn deposit(env: Env, user: Address, amount: i128) {
        user.require_auth();
        env.storage().instance().set(&user, &amount);
        env.events().publish((symbol_short!("deposit"), user), amount);
    }

    pub fn balance(env: Env, user: Address) -> i128 {
        env.storage().instance().get(&user).unwrap_or(0)
    }
}
