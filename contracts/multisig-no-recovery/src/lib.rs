#![no_std]
//! # Multisig Without Recovery — Bug Variant
//!
//! This contract demonstrates the **stuck-state vulnerability**: an N-of-M
//! multisig where signers can be lost (key compromise, death, lost keys) with
//! no recovery path. Once fewer than `threshold` signers remain active, the
//! contract is permanently bricked.
//!
//! Compare with `contracts/multisig/src/lib.rs` which adds a guardian-gated
//! timelock recovery path.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    Address, Bytes, Env, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized    = 1,
    AlreadyInitialized = 2,
    InvalidThreshold  = 3,
    Unauthorized      = 4,
    ProposalNotFound  = 5,
    AlreadyApproved   = 6,
    ThresholdNotMet   = 7,
    AlreadyExecuted   = 8,
}

#[contracttype]
pub enum DataKey {
    Signers,
    Threshold,
    Proposal(Bytes),
    Approval(Bytes, Address),
    // BUG: no RecoveryGuardian, no RecoveryRequest — stuck state is permanent
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ProposalInfo {
    pub approval_count: u32,
    pub executed: bool,
}

#[contract]
pub struct MultisigNoRecovery;

#[contractimpl]
impl MultisigNoRecovery {
    pub fn init(env: Env, signers: Vec<Address>, threshold: u32) {
        if env.storage().instance().has(&DataKey::Signers) {
            env.panic_with_error(Error::AlreadyInitialized);
        }
        if threshold == 0 || threshold as usize > signers.len() as usize {
            env.panic_with_error(Error::InvalidThreshold);
        }
        env.storage().instance().set(&DataKey::Signers, &signers);
        env.storage().instance().set(&DataKey::Threshold, &threshold);
    }

    pub fn approve(env: Env, signer: Address, hash: Bytes) {
        signer.require_auth();
        let signers: Vec<Address> = env.storage().instance()
            .get(&DataKey::Signers)
            .unwrap_or_else(|| env.panic_with_error(Error::NotInitialized));
        if !signers.contains(&signer) {
            env.panic_with_error(Error::Unauthorized);
        }
        let approval_key = DataKey::Approval(hash.clone(), signer.clone());
        if env.storage().temporary().has(&approval_key) {
            env.panic_with_error(Error::AlreadyApproved);
        }
        env.storage().temporary().set(&approval_key, &true);

        let mut info: ProposalInfo = env.storage().temporary()
            .get(&DataKey::Proposal(hash.clone()))
            .unwrap_or(ProposalInfo { approval_count: 0, executed: false });
        info.approval_count += 1;
        env.storage().temporary().set(&DataKey::Proposal(hash), &info);
    }

    // BUG: if signers drop below threshold (lost keys, compromise), this
    // function can never succeed — the contract is permanently stuck.
    pub fn execute(env: Env, hash: Bytes) {
        let threshold: u32 = env.storage().instance()
            .get(&DataKey::Threshold)
            .unwrap_or_else(|| env.panic_with_error(Error::NotInitialized));
        let info: ProposalInfo = env.storage().temporary()
            .get(&DataKey::Proposal(hash.clone()))
            .unwrap_or_else(|| env.panic_with_error(Error::ProposalNotFound));
        if info.executed {
            env.panic_with_error(Error::AlreadyExecuted);
        }
        if info.approval_count < threshold {
            env.panic_with_error(Error::ThresholdNotMet);
        }
        let mut updated = info;
        updated.executed = true;
        env.storage().temporary().set(&DataKey::Proposal(hash), &updated);
        // No recovery path — if we reach stuck state, nothing can help.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, Vec};

    #[test]
    fn test_stuck_state_no_recovery() {
        let env = Env::default();
        env.mock_all_auths();

        let s1 = Address::generate(&env);
        let s2 = Address::generate(&env);
        let s3 = Address::generate(&env);
        let signers = Vec::from_array(&env, [s1.clone(), s2.clone(), s3.clone()]);

        let id = env.register(MultisigNoRecovery, ());
        let c = MultisigNoRecoveryClient::new(&env, &id);
        c.init(&signers, &2u32); // 2-of-3

        // Simulate: s2 and s3 lose their keys — only s1 remains
        // With threshold=2, no proposal can ever reach execution.
        // There is NO recovery function to call.
        // The contract is permanently stuck.
        assert!(c.try_execute(&soroban_sdk::Bytes::from_array(&env, &[0u8; 32])).is_err(),
            "stuck state: execute must fail when threshold cannot be met");
    }
}
