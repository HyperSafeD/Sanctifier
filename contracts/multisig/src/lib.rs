//! # Multisig Wallet Contract
//!
//! M-of-N multisignature wallet on Soroban.  Signers propose, approve, and
//! execute arbitrary cross-contract calls once the approval threshold is met.
//!
//! ## 🔐 Security Disclaimer
//!
//! **Contract:** Multisig Wallet  
//! **Security Level:** Critical  
//! **Audit Required:** true  
//!
//! ⚠️  SECURITY WARNING: This contract has not been audited. Use at your own risk. Deploy only after thorough testing and security review. CRITICAL: Formal verification required.
//!
//! **Testing Requirements:** Requirements: Formal verification, comprehensive audit, stress testing, security review
//!
//! Use this contract only after understanding the risks and implementing appropriate security measures.
//!
//! ## Public Interface (ABI)
//!
//! | Function | Description |
//! |---|---|
//! | [`MultisigWallet::init`] | One-time initialisation with signers and threshold |
//! | [`MultisigWallet::propose`] | Create a new proposal, returns its hash |
//! | [`MultisigWallet::approve`] | Approve an existing proposal |
//! | [`MultisigWallet::execute`] | Execute a proposal once threshold is met |
//! | [`MultisigWallet::cancel`] | Cancel a pending proposal (contract auth required) |
//! | [`MultisigWallet::add_signer`] | Add a signer (contract auth required) |
//! | [`MultisigWallet::remove_signer`] | Remove a signer (contract auth required) |
//! | [`MultisigWallet::set_threshold`] | Update the approval threshold (contract auth required) |
//!
//! ## Error Codes
//!
//! See [`Error`] for the full list of contract error variants.
//!
//! ## Security Considerations
//!
//! - This contract handles multi-signature authorization for potentially valuable assets
//! - Threshold configuration is critical for security - ensure proper M-of-N ratios
//! - All signers should be verified and trustworthy addresses
//! - Proposal execution should be carefully reviewed before approval
//! - Consider time locks for high-value operations
//! - Monitor for unusual proposal patterns or rapid approvals
#![no_std]

use security_disclaimers::{DisclaimerCategory, SecurityLevel};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, xdr::ToXdr, Address, Bytes,
    Env, IntoVal, Symbol, Val, Vec,

    // ── Recovery with timelock (issue #831) ──────────────────────────────────

    /// Set the guardian address that can initiate stuck-state recovery.
    /// Must be called via contract self-auth (i.e. through a passed proposal).
    pub fn set_recovery_guardian(env: Env, guardian: Address) {
        env.current_contract_address().require_auth();
        env.storage().instance().set(&DataKey::RecoveryGuardian, &guardian);
    }

    /// Guardian initiates a recovery: proposes new signers + threshold with a
    /// 7-day timelock. Only one pending recovery at a time.
    pub fn initiate_recovery(
        env: Env,
        guardian: Address,
        new_signers: soroban_sdk::Vec<Address>,
        new_threshold: u32,
    ) {
        guardian.require_auth();
        let stored_guardian: Option<Address> =
            env.storage().instance().get(&DataKey::RecoveryGuardian);
        let stored_guardian = stored_guardian
            .unwrap_or_else(|| env.panic_with_error(Error::NoRecoveryGuardian));
        if guardian != stored_guardian {
            env.panic_with_error(Error::Unauthorized);
        }
        if env.storage().instance().has(&DataKey::RecoveryRequest) {
            env.panic_with_error(Error::RecoveryAlreadyPending);
        }
        if new_threshold == 0 || new_threshold as usize > new_signers.len() as usize {
            env.panic_with_error(Error::InvalidThreshold);
        }
        // 7-day timelock (7 * 24 * 60 * 60 seconds)
        let unlock_at = env.ledger().timestamp() + 7 * 24 * 60 * 60;
        let req = RecoveryRequest { new_signers, new_threshold, unlock_at };
        env.storage().instance().set(&DataKey::RecoveryRequest, &req);
    }

    /// Execute a pending recovery after the timelock has expired.
    /// Anyone can call this once the timelock passes.
    pub fn execute_recovery(env: Env) {
        let req: RecoveryRequest = env
            .storage()
            .instance()
            .get(&DataKey::RecoveryRequest)
            .unwrap_or_else(|| env.panic_with_error(Error::ProposalNotFound));
        if env.ledger().timestamp() < req.unlock_at {
            env.panic_with_error(Error::TimelockActive);
        }
        env.storage().instance().set(&DataKey::Signers, &req.new_signers);
        env.storage().instance().set(&DataKey::Threshold, &req.new_threshold);
        env.storage().instance().remove(&DataKey::RecoveryRequest);
    }

    /// Cancel a pending recovery (guardian or contract self-auth).
    pub fn cancel_recovery(env: Env, caller: Address) {
        caller.require_auth();
        let guardian: Option<Address> = env.storage().instance().get(&DataKey::RecoveryGuardian);
        let is_guardian = guardian.map(|g| g == caller).unwrap_or(false);
        let is_self = caller == env.current_contract_address();
        if !is_guardian && !is_self {
            env.panic_with_error(Error::Unauthorized);
        }
        env.storage().instance().remove(&DataKey::RecoveryRequest);
    }

    /// View the pending recovery request, if any.
    pub fn get_recovery_request(env: Env) -> Option<RecoveryRequest> {
        env.storage().instance().get(&DataKey::RecoveryRequest)
    }

};

#[cfg(test)]
mod test;

/// Errors returned by the multisig wallet contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `init` has not been called yet.
    NotInitialized = 1,
    /// `init` has already been called.
    AlreadyInitialized = 2,
    /// Threshold is zero or exceeds the number of signers.
    InvalidThreshold = 3,
    /// Fewer signers provided than the required threshold.
    InsufficientSigners = 4,
    /// Caller is not a registered signer.
    Unauthorized = 5,
    /// No proposal exists with the given hash.
    ProposalNotFound = 6,
    /// This signer has already approved this proposal.
    AlreadyApproved = 7,
    /// Approval count has not reached the threshold yet.
    ThresholdNotMet = 8,
    /// Proposal has already been executed.
    AlreadyExecuted = 9,
    /// Proposal has already been cancelled.
    AlreadyCancelled = 10,
    /// Supplied arguments are invalid for the requested self-call.
    InvalidArguments = 11,
    /// Timelock has not expired yet.
    TimelockActive = 12,
    /// Recovery guardian is not set.
    NoRecoveryGuardian = 13,
    /// Recovery already initiated.
    RecoveryAlreadyPending = 14,
}

#[contracttype]
pub enum DataKey {
    Signers,
    Threshold,
    Proposal(Bytes),          // Proposal Hash -> Info
    Approval(Bytes, Address), // (Hash, Signer) -> bool
    RecoveryGuardian,
    RecoveryRequest,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ProposalInfo {
    pub approval_count: u32,
    pub executed: bool,
    pub cancelled: bool,
}

/// Pending recovery request (timelock-gated).
#[contracttype]
#[derive(Clone, Debug)]
pub struct RecoveryRequest {
    /// New set of signers to install after the timelock.
    pub new_signers: soroban_sdk::Vec<Address>,
    /// New threshold.
    pub new_threshold: u32,
    /// Ledger timestamp after which the recovery can be executed.
    pub unlock_at: u64,
}

#[contract]
pub struct MultisigWallet;

#[contractimpl]
impl MultisigWallet {
    /// Get security disclaimer for this contract
    pub fn get_security_disclaimer(env: Env, category: DisclaimerCategory) -> soroban_sdk::String {
        security_disclaimers::get_disclaimer(env.clone(), SecurityLevel::Critical, category)
    }

    /// Validate security configuration
    pub fn validate_security_config(env: Env, has_admin: bool, has_upgrade: bool) -> bool {
        security_disclaimers::validate_security_config(
            env,
            SecurityLevel::Critical,
            has_admin,
            has_upgrade,
        )
    }

    /// Initialize the multisig wallet with a list of signers and a threshold.
    pub fn init(env: Env, signers: Vec<Address>, threshold: u32) {
        if env.storage().instance().has(&DataKey::Threshold) {
            env.panic_with_error(Error::AlreadyInitialized);
        }
        if threshold == 0 || threshold > signers.len() {
            env.panic_with_error(Error::InvalidThreshold);
        }

        env.storage().instance().set(&DataKey::Signers, &signers);
        env.storage()
            .instance()
            .set(&DataKey::Threshold, &threshold);
    }

    /// Create a new proposal.
    pub fn propose(
        env: Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        salt: Bytes,
    ) -> Bytes {
        let hash = Self::calculate_hash(&env, &target, &function, &args, &salt);

        if env
            .storage()
            .persistent()
            .has(&DataKey::Proposal(hash.clone()))
        {
            return hash;
        }

        let info = ProposalInfo {
            approval_count: 0,
            executed: false,
            cancelled: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Proposal(hash.clone()), &info);

        env.events().publish(
            (symbol_short!("proposed"), hash.clone()),
            (target, function),
        );

        hash
    }

    /// Approve a proposal.
    pub fn approve(env: Env, signer: Address, hash: Bytes) {
        signer.require_auth();

        let signers: Vec<Address> = env.storage().instance().get(&DataKey::Signers).unwrap();
        if !signers.contains(&signer) {
            env.panic_with_error(Error::Unauthorized);
        }

        let mut info: ProposalInfo = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(hash.clone()))
            .unwrap_or_else(|| env.panic_with_error(Error::ProposalNotFound));

        if info.executed {
            env.panic_with_error(Error::AlreadyExecuted);
        }
        if info.cancelled {
            env.panic_with_error(Error::AlreadyCancelled);
        }

        let approval_key = DataKey::Approval(hash.clone(), signer.clone());
        if env.storage().persistent().has(&approval_key) {
            env.panic_with_error(Error::AlreadyApproved);
        }

        info.approval_count += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(hash.clone()), &info);
        env.storage().persistent().set(&approval_key, &true);

        env.events()
            .publish((symbol_short!("approved"), hash), signer.clone());
    }

    /// Execute a proposal if the threshold is met.
    pub fn execute(
        env: Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        salt: Bytes,
    ) -> Val {
        let hash = Self::calculate_hash(&env, &target, &function, &args, &salt);

        let mut info: ProposalInfo = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(hash.clone()))
            .unwrap_or_else(|| env.panic_with_error(Error::ProposalNotFound));

        let threshold: u32 = env.storage().instance().get(&DataKey::Threshold).unwrap();

        if info.approval_count < threshold {
            env.panic_with_error(Error::ThresholdNotMet);
        }
        if info.executed {
            env.panic_with_error(Error::AlreadyExecuted);
        }
        if info.cancelled {
            env.panic_with_error(Error::AlreadyCancelled);
        }

        info.executed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(hash.clone()), &info);

        // Native routing for self-calls to bypass recursion and authorization issues.
        // This is the most robust way to handle administrative self-governance in Soroban.
        let result = if target == env.current_contract_address() {
            if function == Symbol::new(&env, "add_signer") {
                let signer: Address = args.get(0).unwrap().into_val(&env);
                Self::internal_add_signer(&env, signer);
                ().into_val(&env)
            } else if function == Symbol::new(&env, "remove_signer") {
                let signer: Address = args.get(0).unwrap().into_val(&env);
                Self::internal_remove_signer(&env, signer);
                ().into_val(&env)
            } else if function == Symbol::new(&env, "set_threshold") {
                let threshold: u32 = args.get(0).unwrap().into_val(&env);
                Self::internal_set_threshold(&env, threshold);
                ().into_val(&env)
            } else if function == Symbol::new(&env, "cancel") {
                let hash_to_cancel: Bytes = args.get(0).unwrap().into_val(&env);
                Self::internal_cancel(&env, hash_to_cancel);
                ().into_val(&env)
            } else {
                env.panic_with_error(Error::InvalidArguments);
            }
        } else {
            env.invoke_contract::<Val>(&target, &function, args)
        };

        env.events()
            .publish((symbol_short!("executed"), hash), target);
        result
    }

    /// Public wrapper for cancel (requires contract's own auth for top-level call)
    pub fn cancel(env: Env, hash: Bytes) {
        env.current_contract_address().require_auth();
        Self::internal_cancel(&env, hash);
    }

    pub fn add_signer(env: Env, signer: Address) {
        env.current_contract_address().require_auth();
        Self::internal_add_signer(&env, signer);
    }

    pub fn remove_signer(env: Env, signer: Address) {
        env.current_contract_address().require_auth();
        Self::internal_remove_signer(&env, signer);
    }

    pub fn set_threshold(env: Env, threshold: u32) {
        env.current_contract_address().require_auth();
        Self::internal_set_threshold(&env, threshold);
    }

    // --- Internal Helpers ---

    fn internal_cancel(env: &Env, hash: Bytes) {
        let mut info: ProposalInfo = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(hash.clone()))
            .unwrap_or_else(|| env.panic_with_error(Error::ProposalNotFound));

        if info.executed {
            env.panic_with_error(Error::AlreadyExecuted);
        }

        info.cancelled = true;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(hash.clone()), &info);

        env.events().publish((symbol_short!("cancelled"), hash), ());
    }

    fn internal_add_signer(env: &Env, signer: Address) {
        let mut signers: Vec<Address> = env.storage().instance().get(&DataKey::Signers).unwrap();
        if !signers.contains(&signer) {
            signers.push_back(signer);
            env.storage().instance().set(&DataKey::Signers, &signers);
        }
    }

    fn internal_remove_signer(env: &Env, signer: Address) {
        let mut signers: Vec<Address> = env.storage().instance().get(&DataKey::Signers).unwrap();
        let threshold: u32 = env.storage().instance().get(&DataKey::Threshold).unwrap();

        if let Some(idx) = signers.first_index_of(&signer) {
            if signers.len() <= threshold {
                env.panic_with_error(Error::InvalidThreshold);
            }
            signers.remove(idx);
            env.storage().instance().set(&DataKey::Signers, &signers);
        }
    }

    fn internal_set_threshold(env: &Env, threshold: u32) {
        let signers: Vec<Address> = env.storage().instance().get(&DataKey::Signers).unwrap();
        if threshold == 0 || threshold > signers.len() {
            env.panic_with_error(Error::InvalidThreshold);
        }
        env.storage()
            .instance()
            .set(&DataKey::Threshold, &threshold);
    }

    fn calculate_hash(
        env: &Env,
        target: &Address,
        function: &Symbol,
        args: &Vec<Val>,
        salt: &Bytes,
    ) -> Bytes {
        let mut data = Bytes::new(env);
        data.append(&target.clone().to_xdr(env));
        data.append(&function.clone().to_xdr(env));
        data.append(&args.clone().to_xdr(env));
        data.append(&salt.clone().to_xdr(env));

        env.crypto().sha256(&data).into()
    }
}
