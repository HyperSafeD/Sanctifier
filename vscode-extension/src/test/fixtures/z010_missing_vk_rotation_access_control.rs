//! ⚠️  VULNERABLE FIXTURE — FOR TESTING PURPOSES ONLY. DO NOT DEPLOY.
//!
//! Z010 — Verifying Key Rotation Without Access Control
//!
//! Demonstrates a VK-rotation function that is callable by *any* account
//! because it lacks an `env.current_contract_address().require_auth()` or
//! equivalent admin check. An attacker can swap the verifying key to one for
//! which they know the trapdoor, then forge arbitrary proofs.
//!
//! Rule Z010 must flag `rotate_verifying_key_vulnerable` and must NOT flag
//! `rotate_verifying_key_safe`.
//!
//! See: docs/rules/Z010.md

#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Bytes, Env};

const KEY_VK: &str = "VK";
const KEY_ADMIN: &str = "ADMIN";

#[contract]
pub struct VkRotationFixture;

#[contractimpl]
impl VkRotationFixture {
    /// Initialize the contract with an admin and an initial verifying key.
    pub fn initialize(env: Env, admin: Address, initial_vk: Bytes) {
        admin.require_auth();
        env.storage()
            .instance()
            .set(&symbol_short!(KEY_ADMIN), &admin);
        env.storage()
            .instance()
            .set(&symbol_short!(KEY_VK), &initial_vk);
    }

    // -------------------------------------------------------------------------
    // ❌ VULNERABLE: no access control — anyone can rotate the verifying key.
    // Z010 must flag this function.
    // -------------------------------------------------------------------------
    pub fn rotate_verifying_key_vulnerable(env: Env, new_vk: Bytes) {
        // Missing: admin.require_auth() + admin address lookup
        env.storage()
            .instance()
            .set(&symbol_short!(KEY_VK), &new_vk);
    }

    // -------------------------------------------------------------------------
    // ✅ SAFE: only the stored admin can rotate the VK.
    // Z010 must NOT flag this function.
    // -------------------------------------------------------------------------
    pub fn rotate_verifying_key_safe(env: Env, admin: Address, new_vk: Bytes) {
        // Authenticate the caller as the stored admin before writing.
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&symbol_short!(KEY_ADMIN))
            .expect("not initialized");
        if stored_admin != admin {
            panic!("unauthorized: caller is not the admin");
        }
        admin.require_auth();
        env.storage()
            .instance()
            .set(&symbol_short!(KEY_VK), &new_vk);
    }

    /// Returns the current verifying key.
    pub fn get_verifying_key(env: Env) -> Bytes {
        env.storage()
            .instance()
            .get(&symbol_short!(KEY_VK))
            .expect("not initialized")
    }
}
