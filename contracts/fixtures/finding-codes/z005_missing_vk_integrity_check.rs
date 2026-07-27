//! ⚠️  VULNERABLE FIXTURE — FOR TESTING PURPOSES ONLY. DO NOT DEPLOY.
//!
//! Z005 — Missing Verifying-Key Integrity Check Before Use
//!
//! Once a contract supports verifying-key rotation, the key lives in mutable
//! storage. Access control on the rotation function (Z010) answers "who may write
//! the key" — it does not answer "is the key sitting there right now the one we
//! vetted". A storage-key collision, an unrelated migration, or a compromised
//! admin can leave a hostile key in place, and every later `verify` silently
//! accepts proofs forged under it.
//!
//! The defence costs one hash comparison against a reference committed at
//! deployment.
//!
//! Rule Z005 must flag `verify_vulnerable` and `verify_inline_vulnerable`, and
//! must NOT flag `verify_safe` (hash asserted) or `verify_constant_key`
//! (immutable key — nothing to tamper with at runtime).
//!
//! See: docs/rules/Z005.md

#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Bytes, BytesN, Env, Vec};

// ceremony: perpetual powers-of-tau phase2 contribution #47,
// transcript sha256 3f7a1c04e5b28d9f6a1103bb77c4e2d5081aa93cf4be6710d2ac5f38e91b7c19
const IMMUTABLE_VERIFYING_KEY: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

#[contracttype]
pub enum DataKey {
    /// The rotatable verifying key.
    VerifyingKey,
    /// sha256 of the verifying key that governance last approved.
    VkHash,
}

#[contract]
pub struct VkIntegrityFixture;

#[contractimpl]
impl VkIntegrityFixture {
    // -------------------------------------------------------------------------
    // ❌ VULNERABLE: the key is read out of mutable storage and handed straight
    // to the verifier. Z005 must flag this function.
    // -------------------------------------------------------------------------
    pub fn verify_vulnerable(env: Env, proof: BytesN<256>, inputs: Vec<u64>) -> bool {
        let vk: BytesN<64> = env
            .storage()
            .persistent()
            .get(&DataKey::VerifyingKey)
            .expect("verifying key not set");

        groth16_verify(&env, &vk, &proof, &inputs)
    }

    // -------------------------------------------------------------------------
    // ❌ VULNERABLE: same defect, with the storage read inlined into the call.
    // Z005 must flag this function.
    // -------------------------------------------------------------------------
    pub fn verify_inline_vulnerable(env: Env, proof: BytesN<256>, inputs: Vec<u64>) -> bool {
        groth16_verify(
            &env,
            &env.storage()
                .persistent()
                .get(&DataKey::VerifyingKey)
                .expect("verifying key not set"),
            &proof,
            &inputs,
        )
    }

    // -------------------------------------------------------------------------
    // ✅ SAFE: the stored key is hashed and compared against the reference hash
    // before it is trusted. Z005 must NOT flag this function.
    // -------------------------------------------------------------------------
    pub fn verify_safe(env: Env, proof: BytesN<256>, inputs: Vec<u64>) -> bool {
        let vk: BytesN<64> = env
            .storage()
            .persistent()
            .get(&DataKey::VerifyingKey)
            .expect("verifying key not set");
        let expected_vk_hash: BytesN<32> = env
            .storage()
            .persistent()
            .get(&DataKey::VkHash)
            .expect("verifying key hash not set");

        // Integrity gate: abort if the stored key is not the vetted one.
        assert_eq!(
            env.crypto()
                .sha256(&Bytes::from_slice(&env, &vk.to_array()))
                .to_bytes(),
            expected_vk_hash,
            "verifying key integrity check failed"
        );

        groth16_verify(&env, &vk, &proof, &inputs)
    }

    // -------------------------------------------------------------------------
    // ✅ SAFE: an immutable constant cannot be swapped at runtime, so a runtime
    // integrity check is redundant. Z005 must NOT flag this function.
    // (Provenance of a constant key is Z004's concern instead.)
    // -------------------------------------------------------------------------
    pub fn verify_constant_key(env: Env, proof: BytesN<256>, inputs: Vec<u64>) -> bool {
        groth16_verify_raw(&env, &IMMUTABLE_VERIFYING_KEY, &proof, &inputs)
    }

    // -------------------------------------------------------------------------
    // ✅ SAFE: reads the key but never verifies anything with it.
    // Z005 must NOT flag this function.
    // -------------------------------------------------------------------------
    pub fn get_verifying_key(env: Env) -> BytesN<64> {
        env.storage()
            .persistent()
            .get(&DataKey::VerifyingKey)
            .expect("verifying key not set")
    }
}

// ── Stubs so the fixture is self-contained ───────────────────────────────────

fn groth16_verify(_env: &Env, _vk: &BytesN<64>, _proof: &BytesN<256>, _inputs: &Vec<u64>) -> bool {
    true
}

fn groth16_verify_raw(_env: &Env, _vk: &[u8], _proof: &BytesN<256>, _inputs: &Vec<u64>) -> bool {
    true
}
