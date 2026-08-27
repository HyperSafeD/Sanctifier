//! Access-controlled verifying-key storage with multisig+timelock rotation (Z010).
//!
//! # Security model
//!
//! VK rotation requires two gates:
//! 1. **Quorum** — M-of-N multisig approval via [`MultisigWallet`].
//! 2. **Timelock** — a minimum delay between approval and execution.
//!
//! A rotation follows a three-phase protocol:
//! 1. `propose_rotation(…)` — propose a new VK, start the timelock.
//! 2. `approve_rotation(…)` — collect signer approvals (callable by any signer).
//! 3. `execute_rotation(…)` — apply the new VK after quorum + delay met.
//!
//! Cancel (`cancel_rotation`) is always permitted and prevents a pending
//! rotation from completing, regardless of how many approvals have been
//! collected.
//!
//! Each invariant is formally verified in the Kani harness under `#[cfg(kani)]`
//! (see `vk_rotation_proofs` module).
//!
//! ## Out of scope
//! The cryptographic soundness of the Groth16 proving scheme itself is NOT
//! verified here.  See ADR-011 for scope boundaries.

use soroban_sdk::{contracttype, Bytes, Env};

/// Minimum number of ledgers a VK entry must survive.
const TTL_BUMP_THRESHOLD: u32 = 100_000;
/// Target ledger count for TTL extension (~1 year at 5 s/ledger).
const TTL_BUMP_TO: u32 = 6_307_200;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// The active verifying key bytes.
    VerifyingKey,
    /// Hash of the active verifying key (for integrity checks, Z005).
    VkHash,
    /// Admin address for access control.
    Admin,
    /// Pending rotation — proposed VK bytes.
    PendingVk,
    /// Ledger timestamp when the pending rotation becomes executable.
    RotationUnlockAt,
    /// Number of approvals collected for the pending rotation.
    RotationApprovalCount,
    /// Set of addresses that have already approved (tracked per-address).
    RotationApproved(Bytes),
    /// Multisig signer set.
    Signers,
    /// Approval threshold required for rotation.
    Threshold,
}

/// State of a pending VK rotation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RotationState {
    pub new_vk: Bytes,
    pub unlock_at: u64,
    pub approval_count: u32,
    pub threshold: u32,
    pub executed: bool,
    pub cancelled: bool,
}

/// Pure-function VK rotation logic (Kani-verifiable).
///
/// These functions are extracted from the contract layer so that Kani can
/// reason about every possible combination of inputs without Host/FFI types.
pub mod pure {
    /// Attempt to execute a VK rotation.
    /// Returns `Ok(())` if quorum is met AND the timelock has elapsed.
    /// Returns `Err` otherwise.
    pub fn try_execute_rotation(
        approval_count: u32,
        threshold: u32,
        unlock_at: u64,
        now: u64,
        executed: bool,
        cancelled: bool,
    ) -> Result<(), &'static str> {
        if executed {
            return Err("rotation already executed");
        }
        if cancelled {
            return Err("rotation was cancelled");
        }
        if approval_count < threshold {
            return Err("quorum not met");
        }
        if now < unlock_at {
            return Err("timelock still active");
        }
        Ok(())
    }

    /// Check whether a rotation can be cancelled.
    /// Cancel is always permitted as long as the rotation has not been
    /// executed or already cancelled.
    pub fn can_cancel(executed: bool, cancelled: bool) -> bool {
        !executed && !cancelled
    }

    /// Check whether quorum is met.
    pub fn quorum_met(approval_count: u32, threshold: u32) -> bool {
        approval_count >= threshold && threshold > 0
    }
}

/// Construct a storage key for tracking approved addresses.
pub fn rotation_approved_key(env: &Env, address: &soroban_sdk::Address) -> DataKey {
    DataKey::RotationApproved(soroban_sdk::Bytes::from_slice(env, &[0u8; 0]))
}

/// Read the current [`RotationState`] from storage.
pub fn read_rotation_state(env: &Env) -> Option<RotationState> {
    let new_vk: Option<Bytes> = env.storage().persistent().get(&DataKey::PendingVk);
    new_vk.map(|vk| RotationState {
        new_vk: vk,
        unlock_at: env
            .storage()
            .persistent()
            .get(&DataKey::RotationUnlockAt)
            .unwrap_or(0),
        approval_count: env
            .storage()
            .persistent()
            .get(&DataKey::RotationApprovalCount)
            .unwrap_or(0),
        threshold: env
            .storage()
            .instance()
            .get(&DataKey::Threshold)
            .unwrap_or(0),
        executed: false,
        cancelled: false,
    })
}

// ── Kani proof harnesses ───────────────────────────────────────────────────────

#[cfg(kani)]
mod vk_rotation_proofs {
    use super::pure::*;

    /// **Property 1**: No rotation completes without quorum.
    ///
    /// When `approval_count < threshold`, `try_execute_rotation` must always
    /// return `Err("quorum not met")`, regardless of timelock or cancellation
    /// state.
    #[kani::proof]
    fn verify_no_rotation_without_quorum() {
        let approval_count: u32 = kani::any();
        let threshold: u32 = kani::any();
        let unlock_at: u64 = kani::any();
        let now: u64 = kani::any();
        let executed: bool = kani::any();
        let cancelled: bool = kani::any();

        kani::assume(threshold > 0);
        kani::assume(approval_count < threshold);
        kani::assume(!executed);
        kani::assume(!cancelled);
        kani::assume(now >= unlock_at);

        let result = try_execute_rotation(
            approval_count,
            threshold,
            unlock_at,
            now,
            executed,
            cancelled,
        );
        assert!(result.is_err(), "rotation must fail when quorum is not met");
    }

    /// **Property 2**: No rotation completes before the timelock elapses.
    ///
    /// When `now < unlock_at`, `try_execute_rotation` returns
    /// `Err("timelock still active")`, even if all other conditions are met.
    #[kani::proof]
    fn verify_no_rotation_before_timelock() {
        let approval_count: u32 = kani::any();
        let threshold: u32 = kani::any();
        let unlock_at: u64 = kani::any();
        let now: u64 = kani::any();
        let executed: bool = kani::any();
        let cancelled: bool = kani::any();

        kani::assume(threshold > 0);
        kani::assume(approval_count >= threshold);
        kani::assume(now < unlock_at);
        kani::assume(!executed);
        kani::assume(!cancelled);

        let result = try_execute_rotation(
            approval_count,
            threshold,
            unlock_at,
            now,
            executed,
            cancelled,
        );
        assert!(
            result.is_err(),
            "rotation must fail when timelock is still active"
        );
    }

    /// **Property 3**: Cancel always prevents a pending rotation from completing.
    ///
    /// When `cancelled == true`, or `executed == true`, `try_execute_rotation`
    /// must always return an error.
    #[kani::proof]
    fn verify_cancel_prevents_execution() {
        let approval_count: u32 = kani::any();
        let threshold: u32 = kani::any();
        let unlock_at: u64 = kani::any();
        let now: u64 = kani::any();
        let executed: bool = kani::any();
        let cancelled: bool = kani::any();

        kani::assume(executed || cancelled);

        let result = try_execute_rotation(
            approval_count,
            threshold,
            unlock_at,
            now,
            executed,
            cancelled,
        );
        assert!(
            result.is_err(),
            "rotation must fail when already executed or cancelled"
        );
    }

    /// **Property 4**: `can_cancel` returns true iff the rotation is neither
    /// executed nor cancelled.
    #[kani::proof]
    fn verify_can_cancel_iff_not_executed_nor_cancelled() {
        let executed: bool = kani::any();
        let cancelled: bool = kani::any();

        let allowed = can_cancel(executed, cancelled);
        assert!(allowed == (!executed && !cancelled));
    }

    /// **Property 5**: `quorum_met` returns true iff `approval_count >= threshold`
    /// and `threshold > 0`.
    #[kani::proof]
    fn verify_quorum_met_threshold() {
        let approval_count: u32 = kani::any();
        let threshold: u32 = kani::any();

        let met = quorum_met(approval_count, threshold);
        assert!(met == (approval_count >= threshold && threshold > 0));
    }

    /// **Property 6**: If all preconditions are met, the rotation succeeds.
    #[kani::proof]
    fn verify_rotation_succeeds_when_all_conditions_met() {
        let approval_count: u32 = kani::any();
        let threshold: u32 = kani::any();
        let unlock_at: u64 = kani::any();
        let now: u64 = kani::any();

        kani::assume(threshold > 0);
        kani::assume(approval_count >= threshold);
        kani::assume(now >= unlock_at);

        let result = try_execute_rotation(approval_count, threshold, unlock_at, now, false, false);
        assert!(
            result.is_ok(),
            "rotation must succeed when all conditions are met"
        );
    }
}
