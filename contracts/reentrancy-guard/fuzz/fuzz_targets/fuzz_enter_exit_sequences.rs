#![no_main]

//! Fuzz test for sequences of enter/exit operations.
//!
//! This fuzzer tests that arbitrary sequences of guard operations maintain
//! the reentrancy protection invariant and never reach inconsistent states.
//!
//! # Test Strategy
//!
//! - Generate random sequences of enter/exit operations
//! - Track expected state through each transition
//! - Verify actual state matches expected state
//! - Ensure no crashes or panics occur
//!
//! # Invariants
//!
//! - **State consistency**: Actual state always matches expected state
//! - **No double-entry**: Cannot enter twice without exit
//! - **Graceful failures**: All error conditions are properly handled
//!
//! # Running
//!
//! ```bash
//! cargo fuzz run fuzz_enter_exit_sequences -- -max_len=1024
//! ```

use libfuzzer_sys::fuzz_target;
use reentrancy_guard::{enter_pure, exit_pure, GuardStatus};

fuzz_target!(|data: &[u8]| {
    let mut current_status = GuardStatus::Unlocked;

    // Interpret bytes as sequence of operations:
    // 0 = try enter, 1 = exit, other = no-op
    for &byte in data {
        match byte % 3 {
            0 => {
                // Try to enter
                let result = enter_pure(current_status);
                
                if current_status == GuardStatus::Unlocked {
                    // Should succeed
                    assert!(result.is_ok(), "Enter should succeed from unlocked state");
                    current_status = result.unwrap();
                    assert_eq!(current_status, GuardStatus::Locked, "Should be locked after enter");
                } else {
                    // Should fail (reentrancy detected)
                    assert!(result.is_err(), "Enter should fail from locked state");
                    // Status should remain unchanged
                    assert_eq!(current_status, GuardStatus::Locked, "Status should remain locked");
                }
            }
            1 => {
                // Exit
                current_status = exit_pure();
                assert_eq!(current_status, GuardStatus::Unlocked, "Should be unlocked after exit");
            }
            _ => {
                // No-op: test idempotence of state queries
                let check = GuardStatus::from_u32(current_status as u32);
                assert_eq!(check, current_status, "Status conversion should be idempotent");
            }
        }
    }

    // Final invariant: state machine should always be in a valid state
    assert!(
        current_status == GuardStatus::Locked || current_status == GuardStatus::Unlocked,
        "Final state must be either locked or unlocked"
    );
});
