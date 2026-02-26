#![no_std]

use soroban_sdk::{symbol_short, Env, Symbol};

// ── Pure logic (verified with Kani) ─────────────────────────────────────────────

/// Represents the status of the reentrancy guard.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum GuardStatus {
    Unlocked = 0,
    Locked = 1,
}

impl GuardStatus {
    pub fn from_u32(val: u32) -> Self {
        match val {
            0 => GuardStatus::Unlocked,
            _ => GuardStatus::Locked,
        }
    }
}

/// Core logic for reentrancy protection.
///
/// * `current_status` - The current status in storage.
/// * Returns `Ok(GuardStatus::Locked)` if transition is allowed (Unlocked -> Locked).
/// * Returns `Err("reentrancy detected")` if already locked.
pub fn enter_pure(current_status: GuardStatus) -> Result<GuardStatus, &'static str> {
    if current_status == GuardStatus::Locked {
        return Err("reentrancy detected");
    }
    Ok(GuardStatus::Locked)
}

/// Transition from Locked to Unlocked.
pub fn exit_pure() -> GuardStatus {
    GuardStatus::Unlocked
}

// ── Soroban Contract Integration ────────────────────────────────────────────────

const GUARD_KEY: Symbol = symbol_short!("RE_GRD");

pub struct ReentrancyGuard<'a> {
    env: &'a Env,
}

impl<'a> ReentrancyGuard<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self { env }
    }

    /// Enter a reentrancy-protected section.
    /// Panics if reentrancy is detected.
    pub fn enter(&self) {
        let status: u32 = self.env.storage().instance().get(&GUARD_KEY).unwrap_or(0);
        let current = GuardStatus::from_u32(status);
        
        match enter_pure(current) {
            Ok(new_status) => {
                self.env.storage().instance().set(&GUARD_KEY, &(new_status as u32));
            }
            Err(msg) => panic!("{}", msg),
        }
    }

    /// Exit a reentrancy-protected section.
    pub fn exit(&self) {
        let unlocked = exit_pure();
        self.env.storage().instance().set(&GUARD_KEY, &(unlocked as u32));
    }
}

// ── Kani harnesses ─────────────────────────────────────────────────────────────

#[cfg(kani)]
mod verification {
    use super::*;

    /// **Property**: Cannot enter if already locked.
    #[kani::proof]
    fn verify_enter_fails_when_locked() {
        let result = enter_pure(GuardStatus::Locked);
        assert!(result.is_err());
    }

    /// **Property**: Can enter if unlocked.
    #[kani::proof]
    fn verify_enter_succeeds_when_unlocked() {
        let result = enter_pure(GuardStatus::Unlocked);
        assert!(result.is_ok());
        assert!(result.unwrap() == GuardStatus::Locked);
    }

    /// **Property**: Exit always returns Unlocked.
    #[kani::proof]
    fn verify_exit_always_unlocks() {
        let status = exit_pure();
        assert!(status == GuardStatus::Unlocked);
    }

    /// **Property**: State machine idempotency.
    /// Exhaustively checking all GuardStatus values.
    #[kani::proof]
    fn verify_guard_state_machine() {
        // We model status as u32 to simulate kani::any() more broadly if needed,
        // but here we can just use the enum variant logic.
        let is_locked: bool = kani::any();
        let current = if is_locked { GuardStatus::Locked } else { GuardStatus::Unlocked };

        let result = enter_pure(current);

        if is_locked {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok());
            assert!(result.unwrap() == GuardStatus::Locked);
        }
    }
}
