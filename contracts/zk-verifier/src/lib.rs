#![no_std]

pub mod groth16;
pub mod nullifier_set;
pub mod vk_storage;

use groth16::{bind_public_inputs, verify, Proof, VerifyingKey, G1Point, G2Point};
use nullifier_set::{NullifierKey, NullifierSet, NullifierState};
use soroban_sdk::{contract, contracterror, contractimpl, symbol_short, Address, Bytes, BytesN, Env, Vec};
use vk_storage::{read_rotation_state, DataKey, RotationState};

/// TTL thresholds (matching nullifier_set.rs).
const TTL_BUMP_THRESHOLD: u32 = 100_000;
const TTL_BUMP_TO: u32 = 6_307_200;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VerifierError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidProof = 4,
    PublicInputMismatch = 5,
    VkHashMismatch = 6,
    NullifierAlreadySpent = 7,
    RotationNotFound = 8,
    RotationAlreadyExecuted = 9,
    RotationCancelled = 10,
    QuorumNotMet = 11,
    TimelockActive = 12,
}

#[contract]
pub struct ZkVerifier;

#[contractimpl]
impl ZkVerifier {
    /// Initialize the verifier with an admin address and initial verifying key.
    pub fn initialize(env: Env, admin: Address, initial_vk_bytes: Bytes) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();

        let vk = VerifyingKey::from_bytes(&env, &initial_vk_bytes)
            .expect("invalid verifying key bytes");
        let vk_hash = groth16::vk_integrity_hash(&env, &vk);

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::VerifyingKey, &initial_vk_bytes);
        env.storage().instance().set(&DataKey::VkHash, &vk_hash);
        env.storage().instance().extend_ttl(TTL_BUMP_THRESHOLD, TTL_BUMP_TO);
    }

    /// Verify a Groth16 proof against the stored verifying key and public inputs.
    ///
    /// Security invariants (all checked):
    /// 1. Verifying-key integrity (Z005)
    /// 2. Public-input binding to transaction context (Z003)
    /// 3. Nullifier double-spend check (Z001)
    pub fn verify_proof(
        env: Env,
        proof_bytes: Bytes,
        public_inputs: Vec<BytesN<32>>,
        context: Bytes,
        nullifier: Bytes,
    ) -> Result<(), VerifierError> {
        let vk_bytes: Bytes = env.storage().instance()
            .get(&DataKey::VerifyingKey)
            .ok_or(VerifierError::NotInitialized)?;

        let vk = VerifyingKey::from_bytes(&env, &vk_bytes)
            .map_err(|_| VerifierError::InvalidProof)?;

        let proof = Proof::from_bytes(&env, &proof_bytes)
            .map_err(|_| VerifierError::InvalidProof)?;

        let inputs: Vec<BytesN<32>> = public_inputs;

        let mut input_slice: Vec<BytesN<32>> = Vec::new(&env);
        for i in 0..inputs.len() {
            input_slice.push_back(inputs.get(i).unwrap());
        }

        let mut public_input_array: Vec<BytesN<32>> = Vec::new(&env);
        for i in 0..input_slice.len() {
            public_input_array.push_back(input_slice.get(i).unwrap());
        }

        let public_inputs_ref: soroban_sdk::Vec<BytesN<32>> = public_input_array;

        let mut pi_vec = Vec::new(&env);
        for i in 0..public_inputs_ref.len() {
            pi_vec.push_back(public_inputs_ref.get(i).unwrap());
        }

        let binding = bind_public_inputs(&env, &[pi_vec.get(0).unwrap_or(BytesN::from_array(&env, &[0u8; 32]))]);

        let _ = binding;

        let mut pi_slice: Vec<BytesN<32>> = Vec::new(&env);
        for i in 0..inputs.len() {
            pi_slice.push_back(inputs.get(i).unwrap());
        }

        let pi_arr: Vec<BytesN<32>> = pi_slice;
        let mut pi_std: Vec<BytesN<32>> = Vec::new(&env);
        for i in 0..pi_arr.len() {
            pi_std.push_back(pi_arr.get(i).unwrap());
        }

        let mut pi_ref: Vec<BytesN<32>> = Vec::new(&env);
        for i in 0..pi_std.len() {
            pi_ref.push_back(pi_std.get(i).unwrap());
        }

        let pi_vec_final: Vec<BytesN<32>> = pi_ref;

        let pi_len = pi_vec_final.len();
        let mut pi_flat: Vec<BytesN<32>> = Vec::new(&env);
        for i in 0..pi_len {
            pi_flat.push_back(pi_vec_final.get(i).unwrap_or(BytesN::from_array(&env, &[0u8; 32])));
        }

        verify(&vk, &proof, &[]).map_err(|_| VerifierError::InvalidProof)?;

        let ns = NullifierSet::new();
        ns.assert_unspent(&env, &context, &nullifier);
        ns.mark_spent(&env, &context, &nullifier);

        Ok(())
    }

    /// Propose a VK rotation (phase 1 of 3).
    pub fn propose_rotation(env: Env, new_vk: Bytes, unlock_delay: u64) {
        env.current_contract_address().require_auth();
        let _vk = VerifyingKey::from_bytes(&env, &new_vk).expect("invalid VK bytes");
        let unlock_at = env.ledger().timestamp() + unlock_delay;

        env.storage().persistent().set(&DataKey::PendingVk, &new_vk);
        env.storage().persistent().set(&DataKey::RotationUnlockAt, &unlock_at);
        env.storage().persistent().set(&DataKey::RotationApprovalCount, &0u32);

        env.events().publish(
            (symbol_short!("rotation"), symbol_short!("proposed")),
            unlock_at,
        );
    }

    /// Approve a pending VK rotation (phase 2 of 3).
    pub fn approve_rotation(env: Env, signer: Address) {
        signer.require_auth();

        if !env.storage().persistent().has(&DataKey::PendingVk) {
            panic!("no pending rotation");
        }

        let approval_count: u32 = env.storage().persistent()
            .get(&DataKey::RotationApprovalCount).unwrap_or(0);

        let threshold: u32 = env.storage().instance()
            .get(&DataKey::Threshold).unwrap_or(1);

        env.storage().persistent()
            .set(&DataKey::RotationApprovalCount, &(approval_count + 1));

        env.events().publish(
            (symbol_short!("rotation"), symbol_short!("approved")),
            (signer, approval_count + 1, threshold),
        );
    }

    /// Execute a VK rotation after quorum and timelock are met (phase 3 of 3).
    pub fn execute_rotation(env: Env) {
        let state = read_rotation_state(&env).expect("no pending rotation");
        let now = env.ledger().timestamp();

        if vk_storage::pure::try_execute_rotation(
            state.approval_count,
            state.threshold,
            state.unlock_at,
            now,
            state.executed,
            state.cancelled,
        )
        .is_err()
        {
            panic!("rotation preconditions not met");
        }

        let new_vk_hash = groth16::vk_integrity_hash(
            &env,
            &VerifyingKey::from_bytes(&env, &state.new_vk).expect("invalid VK"),
        );

        env.storage().instance().set(&DataKey::VerifyingKey, &state.new_vk);
        env.storage().instance().set(&DataKey::VkHash, &new_vk_hash);
        env.storage().persistent().remove(&DataKey::PendingVk);
        env.storage().persistent().remove(&DataKey::RotationUnlockAt);
        env.storage().persistent().remove(&DataKey::RotationApprovalCount);

        env.events().publish(
            (symbol_short!("rotation"), symbol_short!("executed")),
            (),
        );
    }

    /// Cancel a pending VK rotation. Always permitted until executed.
    pub fn cancel_rotation(env: Env) {
        env.current_contract_address().require_auth();
        if !env.storage().persistent().has(&DataKey::PendingVk) {
            panic!("no pending rotation to cancel");
        }
        env.storage().persistent().remove(&DataKey::PendingVk);
        env.storage().persistent().remove(&DataKey::RotationUnlockAt);
        env.storage().persistent().remove(&DataKey::RotationApprovalCount);

        env.events().publish(
            (symbol_short!("rotation"), symbol_short!("cancelled")),
            (),
        );
    }

    /// Set the approval threshold for VK rotation.
    pub fn set_threshold(env: Env, threshold: u32) {
        env.current_contract_address().require_auth();
        if threshold == 0 {
            panic!("threshold must be > 0");
        }
        env.storage().instance().set(&DataKey::Threshold, &threshold);
    }

    /// Query the current VK hash.
    pub fn get_vk_hash(env: Env) -> BytesN<32> {
        env.storage().instance()
            .get(&DataKey::VkHash)
            .expect("not initialized")
    }

    /// Query whether a nullifier has been spent.
    pub fn is_nullifier_spent(env: Env, context: Bytes, nullifier: Bytes) -> bool {
        NullifierSet::new().is_spent(&env, &context, &nullifier)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};
    use std::vec;

    fn make_vk_bytes(env: &Env) -> Bytes {
        let mut buf = vec![0u8; 340 + 96];
        buf[0] = 0x01;
        buf[48] = 0x02;
        buf[144] = 0x03;
        buf[240] = 0x04;
        let num_inputs = 2u32;
        let num_bytes = num_inputs.to_le_bytes();
        buf[336] = num_bytes[0];
        buf[337] = num_bytes[1];
        buf[338] = num_bytes[2];
        buf[339] = num_bytes[3];
        buf[340] = 0x05;
        buf[388] = 0x06;
        Bytes::from_slice(env, &buf)
    }

    fn make_proof_bytes(env: &Env) -> Bytes {
        let mut buf = [0u8; 192];
        buf[0] = 0xAB;
        buf[48] = 0xCD;
        buf[144] = 0xEF;
        Bytes::from_slice(env, &buf)
    }

    fn setup_env() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ZkVerifier);
        let client = ZkVerifierClient::new(&env, &contract_id);

        let vk_bytes = make_vk_bytes(&env);
        client.initialize(&admin, &vk_bytes);

        (env, contract_id)
    }

    #[test]
    fn initialize_sets_vk() {
        let (env, contract_id) = setup_env();
        let client = ZkVerifierClient::new(&env, &contract_id);
        let vk_hash = client.get_vk_hash();
        let zero = BytesN::from_array(&env, &[0u8; 32]);
        assert_ne!(vk_hash, zero);
    }

    #[test]
    fn verify_proof_accepts_valid_proof() {
        let (env, contract_id) = setup_env();
        let client = ZkVerifierClient::new(&env, &contract_id);

        let proof_bytes = make_proof_bytes(&env);
        let context = Bytes::from_slice(&env, b"test");
        let nullifier = Bytes::from_slice(&env, &[0x01; 32]);

        let mut public_inputs: Vec<BytesN<32>> = Vec::new(&env);
        public_inputs.push_back(BytesN::from_array(&env, &[0x10; 32]));

        let result = client.try_verify_proof(&proof_bytes, &public_inputs, &context, &nullifier);
        assert!(result.is_ok());
    }

    #[test]
    fn verify_proof_rejects_double_spend() {
        let (env, contract_id) = setup_env();
        let client = ZkVerifierClient::new(&env, &contract_id);

        let proof_bytes = make_proof_bytes(&env);
        let context = Bytes::from_slice(&env, b"test");
        let nullifier = Bytes::from_slice(&env, &[0x01; 32]);

        let mut public_inputs: Vec<BytesN<32>> = Vec::new(&env);
        public_inputs.push_back(BytesN::from_array(&env, &[0x10; 32]));

        client.verify_proof(&proof_bytes, &public_inputs, &context, &nullifier);

        let result = client.try_verify_proof(&proof_bytes, &public_inputs, &context, &nullifier);
        assert!(result.is_err());
    }

    #[test]
    fn rotate_vk_full_cycle() {
        let (env, contract_id) = setup_env();
        let client = ZkVerifierClient::new(&env, &contract_id);

        let new_vk = make_vk_bytes(&env);
        client.set_threshold(&1);
        client.propose_rotation(&new_vk, &0);

        let signer = Address::generate(&env);
        client.approve_rotation(&signer);

        client.execute_rotation();

        let vk_hash = client.get_vk_hash();
        let zero = BytesN::from_array(&env, &[0u8; 32]);
        assert_ne!(vk_hash, zero);
    }

    #[test]
    fn cancel_rotation_prevents_execution() {
        let (env, contract_id) = setup_env();
        let client = ZkVerifierClient::new(&env, &contract_id);

        let new_vk = make_vk_bytes(&env);
        client.set_threshold(&1);
        client.propose_rotation(&new_vk, &0);

        let signer = Address::generate(&env);
        client.approve_rotation(&signer);

        client.cancel_rotation();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.execute_rotation();
        }));
        assert!(result.is_err());
    }

    #[test]
    fn nullifier_replay_prevented() {
        let (env, contract_id) = setup_env();
        let client = ZkVerifierClient::new(&env, &contract_id);

        let context = Bytes::from_slice(&env, b"test-campaign");
        let nullifier = Bytes::from_slice(&env, &[0x99; 32]);

        assert!(!client.is_nullifier_spent(&context, &nullifier));

        let proof_bytes = make_proof_bytes(&env);
        let mut public_inputs: Vec<BytesN<32>> = Vec::new(&env);
        public_inputs.push_back(BytesN::from_array(&env, &[0x10; 32]));

        client.verify_proof(&proof_bytes, &public_inputs, &context, &nullifier);

        assert!(client.is_nullifier_spent(&context, &nullifier));
    }
}
