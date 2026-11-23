// ⚠️  VULNERABLE FIXTURE — FOR TESTING AND EDUCATIONAL PURPOSES ONLY
// Classification: Z001 — Missing Nullifier Check
// This file intentionally omits the nullifier check present in the secure
// reference implementation so that rule Z001 correctly flags it.
// DO NOT deploy this contract; it allows double-spending of ZK proofs.

#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env, Bytes};

/// Groth16 verification key stored in contract instance storage.
#[contracttype]
#[derive(Clone)]
pub struct VerifyingKey {
    pub alpha_g1: Bytes,
    pub beta_g2:  Bytes,
    pub gamma_g2: Bytes,
    pub delta_g2: Bytes,
    pub ic:       Bytes,
}

/// A Groth16 proof supplied by the caller.
#[contracttype]
#[derive(Clone)]
pub struct Proof {
    pub a: Bytes,
    pub b: Bytes,
    pub c: Bytes,
}

const KEY_VK: &str = "VK";

#[contract]
pub struct VulnerableZkVerifier;

#[contractimpl]
impl VulnerableZkVerifier {
    /// Store the verifying key (admin only — omitted here for brevity).
    pub fn init(env: Env, vk: VerifyingKey) {
        env.storage().instance().set(&symbol_short!(KEY_VK), &vk);
    }

    /// Verify a Groth16 proof against stored public inputs.
    ///
    /// # Z001 Vulnerability
    /// The nullifier is accepted as a parameter but is **never checked against
    /// the spent-nullifiers set**.  An attacker can replay the same valid proof
    /// with the same nullifier an unlimited number of times, triggering
    /// double-spend of whatever asset this contract gates.
    ///
    /// The secure version (`z001_secure_nullifier.rs`) adds:
    /// ```rust
    /// nullifier_set.assert_unspent(&env, &nullifier);
    /// nullifier_set.mark_spent(&env, &nullifier);
    /// ```
    /// before granting access.
    pub fn verify(
        env: Env,
        proof: Proof,
        public_inputs: Bytes,
        nullifier: Bytes,  // received but never stored or checked ← Z001
    ) -> bool {
        let vk: VerifyingKey = env
            .storage()
            .instance()
            .get(&symbol_short!(KEY_VK))
            .expect("verifying key not initialised");

        // Pairing check stub — real implementation would call a host-function
        // or off-chain oracle; logic omitted so the fixture compiles on its own.
        let _ = (vk, proof, public_inputs);

        // ❌  Missing: nullifier_set.assert_unspent(&env, &nullifier);
        // ❌  Missing: nullifier_set.mark_spent(&env, &nullifier);
        let _ = nullifier;

        true  // placeholder — Z001 flags this function for the missing checks
    }
}

#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Env as _, Bytes, Env};
    use super::{VulnerableZkVerifier, VulnerableZkVerifierClient, VerifyingKey, Proof};

    fn dummy_bytes(env: &Env) -> Bytes {
        Bytes::from_slice(env, &[0u8; 32])
    }

    #[test]
    fn z001_replay_attack_succeeds_on_vulnerable_contract() {
        let env = Env::default();
        let contract_id = env.register_contract(None, VulnerableZkVerifier);
        let client = VulnerableZkVerifierClient::new(&env, &contract_id);

        let vk = VerifyingKey {
            alpha_g1: dummy_bytes(&env),
            beta_g2:  dummy_bytes(&env),
            gamma_g2: dummy_bytes(&env),
            delta_g2: dummy_bytes(&env),
            ic:       dummy_bytes(&env),
        };
        client.init(&vk);

        let proof = Proof {
            a: dummy_bytes(&env),
            b: dummy_bytes(&env),
            c: dummy_bytes(&env),
        };
        let nullifier = dummy_bytes(&env);

        // First call — expected to succeed on both vulnerable and secure versions.
        assert!(client.verify(&proof, &dummy_bytes(&env), &nullifier));

        // Second call with identical nullifier — succeeds on vulnerable contract
        // (double-spend). A secure contract would panic/return false here.
        assert!(
            client.verify(&proof, &dummy_bytes(&env), &nullifier),
            "Z001: replay with same nullifier was not rejected (double-spend possible)"
        );
    }
}
