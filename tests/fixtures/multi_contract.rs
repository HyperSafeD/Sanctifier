#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

// ── Shared storage types ───────────────────────────────────────────────────────

#[contracttype]
pub enum TokenKey {
    Balance(Address),
    Admin,
}

#[contracttype]
pub enum VaultKey {
    Depositor(Address),
    TotalDeposits,
}

// ── Contract A: simple token ──────────────────────────────────────────────────

#[contract]
pub struct TokenA;

#[contractimpl]
impl TokenA {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&TokenKey::Admin, &admin);
    }

    pub fn balance(env: Env, user: Address) -> i128 {
        env.storage().persistent().get(&TokenKey::Balance(user)).unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let from_bal: i128 =
            env.storage().persistent().get(&TokenKey::Balance(from.clone())).unwrap_or(0);
        let to_bal: i128 =
            env.storage().persistent().get(&TokenKey::Balance(to.clone())).unwrap_or(0);
        env.storage().persistent().set(&TokenKey::Balance(from), &(from_bal - amount));
        env.storage().persistent().set(&TokenKey::Balance(to), &(to_bal + amount));
    }
}

// ── Contract B: vault ─────────────────────────────────────────────────────────

#[contract]
pub struct VaultB;

#[contractimpl]
impl VaultB {
    pub fn deposit(env: Env, user: Address, amount: i128) {
        user.require_auth();
        let current: i128 = env
            .storage()
            .persistent()
            .get(&VaultKey::Depositor(user.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&VaultKey::Depositor(user), &(current + amount));
    }

    pub fn total(env: Env) -> i128 {
        env.storage().persistent().get(&VaultKey::TotalDeposits).unwrap_or(0)
    }
}
