#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, xdr::ToXdr, Address, Bytes,
    Env, Symbol, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    InvalidThreshold = 3,
    InsufficientSigners = 4,
    Unauthorized = 5,
    InvalidTransfer = 6,
    Paused = 7,
}

#[contracttype]
pub enum DataKey {
    Signers,
    Threshold,
    Transfer(Bytes), // Hash of transfer request -> status
    Admin,
    Paused,
}

#[contract]
pub struct BridgeContract;

#[contractimpl]
impl BridgeContract {
    /// Initialize the bridge with a list of signers and a threshold for verification.
    pub fn init(env: Env, admin: Address, signers: Vec<Address>, threshold: u32) {
        if env.storage().instance().has(&DataKey::Threshold) {
            env.panic_with_error(Error::AlreadyInitialized);
        }
        if threshold == 0 || threshold > signers.len() {
            env.panic_with_error(Error::InvalidThreshold);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Signers, &signers);
        env.storage()
            .instance()
            .set(&DataKey::Threshold, &threshold);
        env.storage().instance().set(&DataKey::Paused, &false);
    }

    /// Execute a cross-chain transfer after validating signatures from enough signers.
    /// This is a reference implementation of a secure bridge endpoint.
    pub fn execute_transfer(
        env: Env,
        source_chain: Symbol,
        source_txn: Bytes,
        target_addr: Address,
        amount: u128,
        signers_providing_auth: Vec<Address>,
    ) {
        let is_paused: bool = env.storage().instance().get(&DataKey::Paused).unwrap_or(false);
        if is_paused {
            env.panic_with_error(Error::Paused);
        }

        let transfer_hash =
            Self::calculate_transfer_hash(&env, &source_chain, &source_txn, &target_addr, amount);

        if env
            .storage()
            .persistent()
            .has(&DataKey::Transfer(transfer_hash.clone()))
        {
            // Already processed to prevent double-spending/relay attacks
            return;
        }

        let authorized_signers: Vec<Address> =
            env.storage().instance().get(&DataKey::Signers).unwrap();
        let threshold: u32 = env.storage().instance().get(&DataKey::Threshold).unwrap();

        let mut valid_signers_count = 0;
        let mut processed_signers = Vec::<Address>::new(&env);

        for signer in signers_providing_auth {
            if !authorized_signers.contains(&signer) || processed_signers.contains(&signer) {
                continue;
            }

            // In Soroban, require_auth() handles the cryptographic validation of the caller/signer.
            signer.require_auth();

            valid_signers_count += 1;
            processed_signers.push_back(signer);
        }

        if valid_signers_count < threshold {
            env.panic_with_error(Error::InsufficientSigners);
        }

        // Mark as processed
        env.storage()
            .persistent()
            .set(&DataKey::Transfer(transfer_hash.clone()), &true);

        // Emit bridge event
        env.events().publish(
            (symbol_short!("bridged"), source_chain, source_txn),
            (target_addr, amount),
        );

        // In a real implementation, this would trigger the minting or release of tokens.
    }

    fn calculate_transfer_hash(
        env: &Env,
        source_chain: &Symbol,
        source_txn: &Bytes,
        target_addr: &Address,
        amount: u128,
    ) -> Bytes {
        let mut data = Bytes::new(env);
        data.append(&source_chain.to_xdr(env));
        data.append(&source_txn.clone().to_xdr(env));
        data.append(&target_addr.to_xdr(env));
        data.append(&amount.to_xdr(env));
        env.crypto().sha256(&data).into()
    }

    pub fn pause(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &true);
    }

    pub fn unpause(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &false);
    }
}
