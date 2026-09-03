//! ⚠️  VULNERABLE FIXTURE — FOR TESTING PURPOSES ONLY. DO NOT DEPLOY.
//!
//! Z004 — Unverified or Hardcoded Trusted-Setup Parameters ("Toxic Waste")
//!
//! Groth16 and comparable schemes derive their verifying key from a trusted setup
//! whose secret randomness — the "toxic waste" — must be provably destroyed.
//! Whoever retains it can forge a proof for any false statement. A verifying key
//! pasted into contract source with no pointer to a public ceremony transcript is
//! therefore unauditable: nobody outside the deploying team can tell whether the
//! trapdoor still exists.
//!
//! Rule Z004 flags *provenance*, not cryptography. It must flag
//! `VK_ALPHA_G1_UNDOCUMENTED` and `TRUSTED_SETUP_PARAMS`, and must NOT flag the
//! documented keys below or the unrelated constants.
//!
//! See: docs/rules/Z004.md

#![no_std]
use soroban_sdk::{contract, contractimpl, BytesN, Env};

// -----------------------------------------------------------------------------
// ❌ VULNERABLE: key material with no ceremony reference anywhere near it.
// Z004 must flag this constant.
// -----------------------------------------------------------------------------
const VK_ALPHA_G1_UNDOCUMENTED: [u8; 8] = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x11, 0x22, 0x33];

// -----------------------------------------------------------------------------
// ❌ VULNERABLE: a multi-line setup blob, still undocumented.
// Z004 must flag this constant.
// -----------------------------------------------------------------------------
const TRUSTED_SETUP_PARAMS: [u8; 12] = [
    0xaa, 0xbb, 0xcc, 0xdd, //
    0xee, 0xff, 0x01, 0x02, //
    0x03, 0x04, 0x05, 0x06,
];

// -----------------------------------------------------------------------------
// ✅ SAFE: provenance recorded immediately above the constant — which ceremony,
// which contribution, the transcript hash, and where to fetch it.
// Z004 must NOT flag this constant.
// -----------------------------------------------------------------------------
// ceremony: perpetual powers-of-tau, phase2 contribution #47
// transcript sha256: 3f7a1c04e5b28d9f6a1103bb77c4e2d5081aa93cf4be6710d2ac5f38e91b7c19
// https://ceremony.example.org/transcripts/47.json
const VK_BETA_G2_DOCUMENTED: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

/// Verifying key produced by the Hermez `powersOfTau28_hez` ceremony;
/// attestation and contribution hash published with the circuit release.
/// Z004 must NOT flag this constant.
pub const VERIFYING_KEY: &[u8] = &[0x11, 0x22, 0x33, 0x44];

// -----------------------------------------------------------------------------
// ✅ SAFE: not setup material at all — a storage key and a limit.
// Z004 must NOT flag these.
// -----------------------------------------------------------------------------
const VK_STORAGE_KEY: &str = "VK";
const MAX_PUBLIC_INPUTS: u32 = 16;

#[contract]
pub struct TrustedSetupFixture;

#[contractimpl]
impl TrustedSetupFixture {
    /// Verify against the undocumented key — the reason Z004 exists.
    pub fn verify_undocumented(env: Env, proof: BytesN<256>) -> bool {
        groth16_verify(&env, &VK_ALPHA_G1_UNDOCUMENTED, &proof)
    }

    /// Verify against a key whose ceremony provenance is on record.
    pub fn verify_documented(env: Env, proof: BytesN<256>) -> bool {
        groth16_verify(&env, &VK_BETA_G2_DOCUMENTED, &proof)
    }
}

// ── Stub so the fixture is self-contained ────────────────────────────────────

fn groth16_verify(_env: &Env, _vk: &[u8], _proof: &BytesN<256>) -> bool {
    true
}
