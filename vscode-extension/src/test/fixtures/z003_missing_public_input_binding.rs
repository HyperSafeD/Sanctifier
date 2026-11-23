//! ⚠️  VULNERABLE FIXTURE — FOR TESTING PURPOSES ONLY. DO NOT DEPLOY.
//!
//! Z003 — Missing Public-Input Binding (Proof Malleability)
//!
//! A ZK proof attests only to the statement encoded in its public inputs. When a
//! withdrawal path verifies a proof over `[amount]` but then transfers to a
//! caller-supplied `recipient`, that recipient was never part of what the proof
//! proved. An observer can lift a valid proof out of the mempool, resubmit it with
//! their own address, and redirect the payout — the proof still verifies, because
//! nothing in it ever mentioned the original recipient.
//!
//! Rule Z003 must flag `withdraw_vulnerable` and must NOT flag
//! `withdraw_safe` (recipient bound via a hash) or `withdraw_safe_direct`
//! (recipient passed straight to the verifier).
//!
//! See: docs/rules/Z003.md

#![no_std]
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Vec};

#[contract]
pub struct PublicInputBindingFixture;

#[contractimpl]
impl PublicInputBindingFixture {
    // -------------------------------------------------------------------------
    // ❌ VULNERABLE: `recipient` drives the transfer but never enters the public
    // inputs, so the proof does not commit to it. Z003 must flag this function.
    // -------------------------------------------------------------------------
    pub fn withdraw_vulnerable(env: Env, proof: BytesN<256>, recipient: Address, amount: i128) {
        // Only the amount is bound — the destination is free for the taker.
        let public_inputs = Vec::from_array(&env, [amount as u64]);
        verify_proof(&env, &proof, &public_inputs);

        transfer_out(&env, &recipient, amount);
    }

    // -------------------------------------------------------------------------
    // ✅ SAFE: the recipient is hashed into a field element and included in the
    // public inputs, so the circuit commits to this exact destination.
    // Z003 must NOT flag this function.
    // -------------------------------------------------------------------------
    pub fn withdraw_safe(env: Env, proof: BytesN<256>, recipient: Address, amount: i128) {
        let recipient_hash = hash_address(&env, &recipient);
        let public_inputs = Vec::from_array(&env, [recipient_hash, amount as u64]);
        verify_proof(&env, &proof, &public_inputs);

        transfer_out(&env, &recipient, amount);
    }

    // -------------------------------------------------------------------------
    // ✅ SAFE: the verifier receives the recipient and amount directly.
    // Z003 must NOT flag this function.
    // -------------------------------------------------------------------------
    pub fn withdraw_safe_direct(env: Env, proof: BytesN<256>, recipient: Address, amount: i128) {
        groth16_verify(&env, &proof, &recipient, &amount);

        transfer_out(&env, &recipient, amount);
    }

    // -------------------------------------------------------------------------
    // ✅ SAFE: nothing security-relevant survives the verification, so there is
    // no value an attacker could redirect. Z003 must NOT flag this function.
    // -------------------------------------------------------------------------
    pub fn attest(env: Env, proof: BytesN<256>, recipient: Address) {
        let public_inputs = Vec::from_array(&env, [1u64]);
        verify_proof(&env, &proof, &public_inputs);
    }
}

// ── Stubs so the fixture is self-contained ───────────────────────────────────

fn verify_proof(_env: &Env, _proof: &BytesN<256>, _inputs: &Vec<u64>) -> bool {
    true
}

fn groth16_verify(_env: &Env, _proof: &BytesN<256>, _recipient: &Address, _amount: &i128) -> bool {
    true
}

fn hash_address(_env: &Env, _addr: &Address) -> u64 {
    0
}

fn transfer_out(_env: &Env, _to: &Address, _amount: i128) {}
