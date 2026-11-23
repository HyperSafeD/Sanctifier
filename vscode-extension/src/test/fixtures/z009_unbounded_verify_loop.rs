//! ⚠️  VULNERABLE FIXTURE — FOR TESTING PURPOSES ONLY. DO NOT DEPLOY.
//!
//! Z009 — Unbounded Proof Verification Loop (DoS)
//!
//! Demonstrates a batch-claim function that iterates over a caller-supplied
//! `Vec<Bytes>` of proofs without bounding its length. Because proof
//! verification is computationally expensive, an attacker can submit a very
//! large batch to exhaust the contract's CPU instructions budget and cause
//! a denial-of-service.
//!
//! Rule Z009 must flag `batch_claim_vulnerable` and must NOT flag
//! `batch_claim_safe` (which caps the batch at MAX_BATCH_SIZE).
//!
//! See: docs/rules/Z009.md

#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Bytes, Env, Vec};

/// Maximum proofs allowed in one batch — enforced by the safe variant.
const MAX_BATCH_SIZE: u32 = 100;

#[contract]
pub struct UnboundedVerifyLoopFixture;

#[contractimpl]
impl UnboundedVerifyLoopFixture {
    // -------------------------------------------------------------------------
    // ❌ VULNERABLE: iterates an unbounded caller-supplied proof list.
    // A large batch exhausts CPU budget — Z009 must flag this function.
    // -------------------------------------------------------------------------
    pub fn batch_claim_vulnerable(
        env: Env,
        claimant: Address,
        proofs: Vec<Bytes>,
    ) -> u32 {
        claimant.require_auth();
        let mut verified: u32 = 0;

        // ❌ No length cap: attacker can pass proofs.len() == 10_000
        for i in 0..proofs.len() {
            let proof = proofs.get(i).unwrap();
            if Self::verify_proof(&env, &proof) {
                verified += 1;
            }
        }
        verified
    }

    // -------------------------------------------------------------------------
    // ✅ SAFE: caps the batch length before entering the verification loop.
    // Z009 must NOT flag this function.
    // -------------------------------------------------------------------------
    pub fn batch_claim_safe(
        env: Env,
        claimant: Address,
        proofs: Vec<Bytes>,
    ) -> u32 {
        claimant.require_auth();

        // ✅ Reject oversized batches immediately.
        if proofs.len() > MAX_BATCH_SIZE {
            panic!("batch too large: max {} proofs per call", MAX_BATCH_SIZE);
        }

        let mut verified: u32 = 0;
        for i in 0..proofs.len() {
            let proof = proofs.get(i).unwrap();
            if Self::verify_proof(&env, &proof) {
                verified += 1;
            }
        }
        verified
    }

    // -------------------------------------------------------------------------
    // Internal stub: real implementation would call a ZK verifier host function.
    // -------------------------------------------------------------------------
    fn verify_proof(_env: &Env, proof: &Bytes) -> bool {
        // Simulate non-trivial CPU work proportional to proof size.
        !proof.is_empty()
    }

    /// Returns the configured batch size cap.
    pub fn max_batch_size(_env: Env) -> u32 {
        MAX_BATCH_SIZE
    }

    /// Emits the current ledger sequence used for domain separation in proofs.
    pub fn current_ledger(env: Env) -> u32 {
        env.ledger().sequence()
    }
}
