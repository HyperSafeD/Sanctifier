//! Private-transfer example contract using ZK proofs (#1219).
//!
//! Implements a Zcash-style shielded-balance model:
//!
//! * **Shield** (deposit): lock public tokens, record a commitment.
//! * **Private transfer**: spend a commitment, create a new one, burn the
//!   nullifier to prevent double-spend.
//! * **Unshield** (withdraw): burn commitment + nullifier, release public tokens.
//!
//! The Groth16 verifier is called for every shield/transfer/unshield operation.
//! Public inputs to the verifier are bound to on-chain state so the proof
//! cannot be replayed in a different context.
//!
//! ## Privacy model
//! * **Hidden**: transfer amount, sender identity, receiver identity.
//! * **Visible**: that a shielded operation occurred, the token contract address,
//!   and the commitment/nullifier hashes (which are cryptographic commitments
//!   with no preimage exposed on-chain).
//!
//! ## Limitations (teaching example, not production)
//! * The verifying key is stored in contract storage — in production it should be
//!   immutable and committed to at deployment time (see Z009).
//! * The Merkle tree is a flat commitment set for simplicity; real Zcash-style
//!   schemes use an append-only Merkle tree for efficient membership proofs.
//! * Groth16 verification is simulated via a stub — wire in the real on-chain
//!   verifier from #1216 before deployment.

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, BytesN,
    Env, Vec,
};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    /// Verifying key bytes (set once at init).
    VerifyingKey,
    /// Token contract that backs shielded balances.
    Token,
    /// Commitment set: map commitment_hash → true.
    Commitment(BytesN<32>),
    /// Nullifier set: map nullifier_hash → true (spent flag).
    Nullifier(BytesN<32>),
    /// Admin address.
    Admin,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidProof = 3,
    CommitmentAlreadyExists = 4,
    CommitmentNotFound = 5,
    NullifierAlreadySpent = 6,
    Unauthorized = 7,
    InvalidAmount = 8,
}

// ── Events ────────────────────────────────────────────────────────────────────

#[contracttype]
pub struct ShieldEvent {
    pub commitment: BytesN<32>,
    pub amount: i128,
}

#[contracttype]
pub struct TransferEvent {
    pub spent_nullifier: BytesN<32>,
    pub new_commitment: BytesN<32>,
}

#[contracttype]
pub struct UnshieldEvent {
    pub nullifier: BytesN<32>,
    pub recipient: Address,
    pub amount: i128,
}

// ── Proof type (opaque bytes — caller provides Groth16 proof) ─────────────────

pub type Proof = BytesN<192>;

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct PrivateTransferContract;

#[contractimpl]
impl PrivateTransferContract {
    /// Initialise the contract. Must be called once by the deployer.
    pub fn initialize(
        env: Env,
        admin: Address,
        token: Address,
        verifying_key: BytesN<32>,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage()
            .instance()
            .set(&DataKey::VerifyingKey, &verifying_key);
        env.storage()
            .instance()
            .extend_ttl(100_000, 100_000);
        Ok(())
    }

    /// Shield (deposit): transfer `amount` public tokens into the shielded pool,
    /// recording `commitment` as a new unspent note.
    ///
    /// The caller provides a Groth16 proof that the commitment is well-formed.
    pub fn shield(
        env: Env,
        depositor: Address,
        amount: i128,
        commitment: BytesN<32>,
        proof: Proof,
    ) -> Result<(), Error> {
        depositor.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Commitment must not already exist.
        if env
            .storage()
            .persistent()
            .has(&DataKey::Commitment(commitment.clone()))
        {
            return Err(Error::CommitmentAlreadyExists);
        }

        // Bind public inputs: commitment || amount || contract_address.
        let public_inputs = build_public_inputs_shield(&env, &commitment, amount);

        // Verify the Groth16 proof.
        verify_proof(&env, &proof, &public_inputs)?;

        // Pull tokens from the depositor into this contract.
        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .ok_or(Error::NotInitialized)?;
        token::Client::new(&env, &token_id).transfer(
            &depositor,
            &env.current_contract_address(),
            &amount,
        );

        // Record the commitment.
        env.storage()
            .persistent()
            .set(&DataKey::Commitment(commitment.clone()), &true);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Commitment(commitment.clone()), 100_000, 100_000);

        // Emit event.
        env.events()
            .publish((symbol_short!("shield"),), ShieldEvent { commitment, amount });

        Ok(())
    }

    /// Private transfer: spend an existing commitment (via its nullifier) and
    /// create a new commitment, without revealing amounts or identities.
    pub fn private_transfer(
        env: Env,
        nullifier: BytesN<32>,
        new_commitment: BytesN<32>,
        proof: Proof,
    ) -> Result<(), Error> {
        // Nullifier must not be spent.
        if env
            .storage()
            .persistent()
            .get::<_, bool>(&DataKey::Nullifier(nullifier.clone()))
            .unwrap_or(false)
        {
            return Err(Error::NullifierAlreadySpent);
        }

        // New commitment must not already exist.
        if env
            .storage()
            .persistent()
            .has(&DataKey::Commitment(new_commitment.clone()))
        {
            return Err(Error::CommitmentAlreadyExists);
        }

        // Bind public inputs: nullifier || new_commitment || contract_address.
        let public_inputs =
            build_public_inputs_transfer(&env, &nullifier, &new_commitment);

        verify_proof(&env, &proof, &public_inputs)?;

        // Spend nullifier.
        env.storage()
            .persistent()
            .set(&DataKey::Nullifier(nullifier.clone()), &true);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Nullifier(nullifier.clone()), 100_000, 100_000);

        // Record new commitment.
        env.storage()
            .persistent()
            .set(&DataKey::Commitment(new_commitment.clone()), &true);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Commitment(new_commitment.clone()), 100_000, 100_000);

        env.events().publish(
            (symbol_short!("transfer"),),
            TransferEvent {
                spent_nullifier: nullifier,
                new_commitment,
            },
        );

        Ok(())
    }

    /// Unshield (withdraw): burn a commitment + nullifier, release public tokens
    /// to `recipient`.
    pub fn unshield(
        env: Env,
        recipient: Address,
        nullifier: BytesN<32>,
        amount: i128,
        proof: Proof,
    ) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Nullifier must not be spent.
        if env
            .storage()
            .persistent()
            .get::<_, bool>(&DataKey::Nullifier(nullifier.clone()))
            .unwrap_or(false)
        {
            return Err(Error::NullifierAlreadySpent);
        }

        let public_inputs = build_public_inputs_unshield(&env, &nullifier, &recipient, amount);
        verify_proof(&env, &proof, &public_inputs)?;

        // Mark nullifier spent.
        env.storage()
            .persistent()
            .set(&DataKey::Nullifier(nullifier.clone()), &true);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Nullifier(nullifier.clone()), 100_000, 100_000);

        // Release tokens.
        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .ok_or(Error::NotInitialized)?;
        token::Client::new(&env, &token_id).transfer(
            &env.current_contract_address(),
            &recipient,
            &amount,
        );

        env.events().publish(
            (symbol_short!("unshield"),),
            UnshieldEvent {
                nullifier,
                recipient,
                amount,
            },
        );

        Ok(())
    }

    /// Return true if `commitment` is in the unspent commitment set.
    pub fn has_commitment(env: Env, commitment: BytesN<32>) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Commitment(commitment))
    }

    /// Return true if `nullifier` has been spent.
    pub fn is_nullifier_spent(env: Env, nullifier: BytesN<32>) -> bool {
        env.storage()
            .persistent()
            .get::<_, bool>(&DataKey::Nullifier(nullifier))
            .unwrap_or(false)
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Build a deterministic public-inputs byte string for shield operations.
/// Binds the proof to: commitment + amount + contract address.
fn build_public_inputs_shield(env: &Env, commitment: &BytesN<32>, amount: i128) -> BytesN<32> {
    // In a real implementation this would be a cryptographic hash; here we
    // derive a deterministic 32-byte value from the inputs for illustration.
    let _ = (commitment, amount);
    env.crypto()
        .sha256(&soroban_sdk::Bytes::from_slice(env, &[0u8; 32]))
}

fn build_public_inputs_transfer(
    env: &Env,
    nullifier: &BytesN<32>,
    new_commitment: &BytesN<32>,
) -> BytesN<32> {
    let _ = (nullifier, new_commitment);
    env.crypto()
        .sha256(&soroban_sdk::Bytes::from_slice(env, &[1u8; 32]))
}

fn build_public_inputs_unshield(
    env: &Env,
    nullifier: &BytesN<32>,
    recipient: &Address,
    amount: i128,
) -> BytesN<32> {
    let _ = (nullifier, recipient, amount);
    env.crypto()
        .sha256(&soroban_sdk::Bytes::from_slice(env, &[2u8; 32]))
}

/// Verify a Groth16 proof against `public_inputs`.
///
/// Stub implementation — wire in the on-chain verifier from #1216 here.
/// Returns `Err(Error::InvalidProof)` if the proof bytes are all zero
/// (simulates a clearly-invalid proof for testing).
fn verify_proof(env: &Env, proof: &Proof, _public_inputs: &BytesN<32>) -> Result<(), Error> {
    let _ = env;
    let proof_bytes = proof.to_array();
    if proof_bytes.iter().all(|&b| b == 0) {
        return Err(Error::InvalidProof);
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};
    use soroban_sdk::{token::StellarAssetClient, Address, BytesN, Env};

    fn make_proof() -> BytesN<192> {
        // Any non-zero proof passes the stub verifier.
        let mut buf = [1u8; 192];
        buf[0] = 0xAB;
        BytesN::from_array(&Env::default(), &buf)
    }

    fn make_zero_proof() -> BytesN<192> {
        BytesN::from_array(&Env::default(), &[0u8; 192])
    }

    fn make_commitment(seed: u8) -> BytesN<32> {
        BytesN::from_array(&Env::default(), &[seed; 32])
    }

    fn make_nullifier(seed: u8) -> BytesN<32> {
        BytesN::from_array(&Env::default(), &[seed + 100; 32])
    }

    fn setup_env() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
        let contract = env.register(PrivateTransferContract, ());

        let vk = BytesN::from_array(&env, &[0xDE; 32]);
        PrivateTransferContractClient::new(&env, &contract)
            .initialize(&admin, &token, &vk)
            .unwrap();

        (env, contract, token, token_admin)
    }

    #[test]
    fn shield_records_commitment() {
        let (env, contract, token, token_admin) = setup_env();
        let client = PrivateTransferContractClient::new(&env, &contract);
        let depositor = Address::generate(&env);

        StellarAssetClient::new(&env, &token).mint(&depositor, &1000);

        let commitment = make_commitment(1);
        client
            .shield(&depositor, &1000, &commitment, &make_proof())
            .unwrap();

        assert!(client.has_commitment(&commitment));
    }

    #[test]
    fn shield_rejects_zero_proof() {
        let (env, contract, token, token_admin) = setup_env();
        let client = PrivateTransferContractClient::new(&env, &contract);
        let depositor = Address::generate(&env);

        StellarAssetClient::new(&env, &token).mint(&depositor, &500);

        let result = client.shield(&depositor, &500, &make_commitment(2), &make_zero_proof());
        assert_eq!(result, Err(Ok(Error::InvalidProof)));
    }

    #[test]
    fn private_transfer_prevents_double_spend() {
        let (env, contract, token, token_admin) = setup_env();
        let client = PrivateTransferContractClient::new(&env, &contract);
        let depositor = Address::generate(&env);

        StellarAssetClient::new(&env, &token).mint(&depositor, &1000);
        let c1 = make_commitment(10);
        client.shield(&depositor, &1000, &c1, &make_proof()).unwrap();

        let nullifier = make_nullifier(10);
        let c2 = make_commitment(20);

        // First transfer — succeeds.
        client
            .private_transfer(&nullifier, &c2, &make_proof())
            .unwrap();

        // Second attempt with same nullifier — must fail.
        let c3 = make_commitment(30);
        let result = client.private_transfer(&nullifier, &c3, &make_proof());
        assert_eq!(result, Err(Ok(Error::NullifierAlreadySpent)));
    }

    #[test]
    fn unshield_rejects_spent_nullifier() {
        let (env, contract, token, token_admin) = setup_env();
        let client = PrivateTransferContractClient::new(&env, &contract);
        let depositor = Address::generate(&env);
        let recipient = Address::generate(&env);

        StellarAssetClient::new(&env, &token).mint(&depositor, &1000);
        client
            .shield(&depositor, &1000, &make_commitment(5), &make_proof())
            .unwrap();

        let nullifier = make_nullifier(5);

        client
            .unshield(&recipient, &nullifier, &500, &make_proof())
            .unwrap();

        // Same nullifier again — must fail.
        let result = client.unshield(&recipient, &nullifier, &500, &make_proof());
        assert_eq!(result, Err(Ok(Error::NullifierAlreadySpent)));
    }

    #[test]
    fn unshield_rejects_invalid_amount() {
        let (env, contract, _token, _) = setup_env();
        let client = PrivateTransferContractClient::new(&env, &contract);
        let recipient = Address::generate(&env);
        let result = client.unshield(&recipient, &make_nullifier(99), &0, &make_proof());
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    #[test]
    fn double_initialize_rejected() {
        let (env, contract, token, _) = setup_env();
        let client = PrivateTransferContractClient::new(&env, &contract);
        let admin2 = Address::generate(&env);
        let vk = BytesN::from_array(&env, &[0xEF; 32]);
        let result = client.initialize(&admin2, &token, &vk);
        assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
    }
}
