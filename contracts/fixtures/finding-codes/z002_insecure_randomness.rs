//! Z002 — Predictable randomness used as commitment secret (ZK verifier fixture).
//!
//! ⚠️  THIS FILE IS AN INTENTIONALLY VULNERABLE FIXTURE.
//! DO NOT USE IN PRODUCTION. It exists solely to exercise rule Z002 and to
//! serve as a concrete failing example for the test suite.
//!
//! ## Vulnerability
//! Using `env.ledger().timestamp()` as the blinding/randomness factor in a
//! Pedersen-style commitment is insecure. The ledger timestamp is publicly
//! observable and validator-nudgeable within a short window, so an adversary
//! can enumerate candidate timestamps around the claimed submission block to
//! reconstruct the committed value without knowing the prover's secret.
//!
//! ## Rule Reference: Z002
//! Triggered when on-chain state (timestamp, ledger sequence, block hash) is
//! used as the sole source of randomness in a ZK commitment or hash-based
//! hiding scheme.
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Bytes, BytesN, Env};

// ── Vulnerable contract ───────────────────────────────────────────────────────

#[contract]
pub struct InsecureCommitmentFixture;

#[contractimpl]
impl InsecureCommitmentFixture {
    /// ❌ VULNERABLE (Z002): commits to `value` using ledger timestamp as the
    /// blinding factor.  The timestamp is public and enumerable — an observer
    /// who knows the approximate submission block can brute-force the opening.
    pub fn commit(env: Env, value: u64) -> BytesN<32> {
        // Predictable: validators can manipulate timestamp within ~1 s window.
        let randomness = env.ledger().timestamp();

        // Naive commitment: H(value || randomness) — randomness is not secret.
        let mut buf = Bytes::new(&env);
        buf.push_back((value >> 56) as u8);
        buf.push_back((value >> 48) as u8);
        buf.push_back((value >> 40) as u8);
        buf.push_back((value >> 32) as u8);
        buf.push_back((value >> 24) as u8);
        buf.push_back((value >> 16) as u8);
        buf.push_back((value >> 8) as u8);
        buf.push_back(value as u8);
        buf.push_back((randomness >> 56) as u8);
        buf.push_back((randomness >> 48) as u8);
        buf.push_back((randomness >> 40) as u8);
        buf.push_back((randomness >> 32) as u8);
        buf.push_back((randomness >> 24) as u8);
        buf.push_back((randomness >> 16) as u8);
        buf.push_back((randomness >> 8) as u8);
        buf.push_back(randomness as u8);

        env.crypto().sha256(&buf)
    }

    /// ❌ VULNERABLE (Z002): uses ledger sequence number as a nullifier nonce.
    /// Ledger sequence is public state — any party can derive the nullifier
    /// without knowledge of the private witness.
    pub fn derive_nullifier(env: Env, secret: u64) -> BytesN<32> {
        let nonce = env.ledger().sequence() as u64; // predictable on-chain value

        let mut buf = Bytes::new(&env);
        buf.push_back((secret >> 56) as u8);
        buf.push_back((secret >> 24) as u8);
        buf.push_back(secret as u8);
        buf.push_back((nonce >> 24) as u8);
        buf.push_back(nonce as u8);

        env.crypto().sha256(&buf)
    }
}

// ── Safe reference implementation ────────────────────────────────────────────

#[contract]
pub struct SecureCommitmentFixture;

#[contractimpl]
impl SecureCommitmentFixture {
    /// ✅ SAFE: the blinding factor (`blinding`) is supplied by the prover
    /// off-chain and is never derived from observable on-chain state.
    /// Z002 is NOT triggered because the randomness is an explicit private input.
    pub fn commit(env: Env, value: u64, blinding: BytesN<32>) -> BytesN<32> {
        let mut buf = Bytes::new(&env);
        buf.push_back((value >> 56) as u8);
        buf.push_back((value >> 48) as u8);
        buf.push_back((value >> 40) as u8);
        buf.push_back((value >> 32) as u8);
        buf.push_back((value >> 24) as u8);
        buf.push_back((value >> 16) as u8);
        buf.push_back((value >> 8) as u8);
        buf.push_back(value as u8);
        // Append the caller-supplied off-chain randomness.
        buf.extend_from_bytes(&blinding);
        env.crypto().sha256(&buf)
    }

    /// ✅ SAFE: nullifier nonce is provided by the prover, not read from ledger state.
    pub fn derive_nullifier(env: Env, secret: u64, nonce: BytesN<32>) -> BytesN<32> {
        let mut buf = Bytes::new(&env);
        buf.push_back((secret >> 56) as u8);
        buf.push_back(secret as u8);
        buf.extend_from_bytes(&nonce);
        env.crypto().sha256(&buf)
    }

    /// ✅ SAFE: timestamp used only for expiry check, not as randomness.
    pub fn is_expired(env: Env, deadline: u64) -> bool {
        env.ledger().timestamp() > deadline
    }

    /// ✅ SAFE: stores the commitment but the blinding factor originates off-chain.
    pub fn store_commitment(env: Env, commitment: BytesN<32>) {
        env.storage()
            .persistent()
            .set(&symbol_short!("COMMIT"), &commitment);
    }
}
