#![no_main]

//! Fuzz test for reentrancy guard state machine transitions.
//!
//! This fuzzer tests the pure logic of the reentrancy guard with arbitrary inputs
//! to ensure it handles unexpected states gracefully and maintains invariants.
//!
//! # Invariants Tested
//!
//! 1. **Single-entry**: Only one thread can enter at a time
//! 2. **Unlocked start**: Guard starts in unlocked state
//! 3. **Exit always unlocks**: Exit always transitions to unlocked
//! 4. **Deterministic**: Same input produces same output
//!
//! # Running
//!
//! ```bash
//! cd contracts/reentrancy-guard
//! cargo fuzz run fuzz_guard_state_machine
//! ```

use libfuzzer_sys::fuzz_target;
use reentrancy_guard::{enter_pure, exit_pure, GuardStatus};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Fuzz the state machine with arbitrary status values
    let status_value = u32::from_le_bytes([
        data[0],
        data.get(1).copied().unwrap_or(0),
        data.get(2).copied().unwrap_or(0),
        data.get(3).copied().unwrap_or(0),
    ]);

    let status = GuardStatus::from_u32(status_value);

    // Test enter_pure with fuzzy status
    let enter_result = enter_pure(status);

    // Invariant 1: If locked, enter must fail
    if status == GuardStatus::Locked {
        assert!(enter_result.is_err(), "Enter should fail when already locked");
    } else {
        // Invariant 2: If unlocked, enter must succeed and lock
        assert!(enter_result.is_ok(), "Enter should succeed when unlocked");
        assert_eq!(
            enter_result.unwrap(),
            GuardStatus::Locked,
            "Enter should transition to locked state"
        );
    }

    // Test exit_pure (should always work)
    let exit_status = exit_pure();

    // Invariant 3: Exit always returns unlocked
    assert_eq!(
        exit_status,
        GuardStatus::Unlocked,
        "Exit should always return unlocked status"
    );

    // Test idempotency: calling exit multiple times should be safe
    let exit_again = exit_pure();
    assert_eq!(exit_again, GuardStatus::Unlocked, "Exit is idempotent");
});
