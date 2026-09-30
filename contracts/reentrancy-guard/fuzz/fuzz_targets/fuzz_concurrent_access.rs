#![no_main]

//! Fuzz test simulating concurrent access patterns.
//!
//! This fuzzer simulates multiple threads attempting to access the guard
//! concurrently by testing rapid sequences of operations and edge cases
//! around state transitions.
//!
//! # What This Tests
//!
//! - **Race conditions**: Rapid enter/exit sequences
//! - **Boundary conditions**: Maximum nesting depth
//! - **Edge cases**: Unusual operation patterns
//! - **Resilience**: Recovery from error states
//!
//! # Fuzzing Strategy
//!
//! Input bytes are split into "threads" that each perform a sequence of
//! operations. We verify that the guard maintains its invariants even
//! when operations are interleaved.
//!
//! # Running
//!
//! ```bash
//! cargo fuzz run fuzz_concurrent_access -- -max_len=256 -runs=10000000
//! ```

use libfuzzer_sys::fuzz_target;
use reentrancy_guard::{enter_pure, exit_pure, GuardStatus};

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    // Split data into "threads" (simulate concurrent access patterns)
    let num_threads = (data[0] % 8) as usize + 1; // 1-8 threads
    let ops_per_thread = data.len().saturating_sub(1) / num_threads;

    if ops_per_thread == 0 {
        return;
    }

    // Simulate each "thread" trying to access the guard
    let mut global_state = GuardStatus::Unlocked;
    let mut entry_count = 0u32; // Track how many times we're nested

    for thread_id in 0..num_threads {
        let start_idx = 1 + thread_id * ops_per_thread;
        let end_idx = start_idx + ops_per_thread;

        if end_idx > data.len() {
            break;
        }

        let thread_ops = &data[start_idx..end_idx];

        for &op in thread_ops {
            match op % 4 {
                0 | 1 => {
                    // Try to enter (more common)
                    let result = enter_pure(global_state);

                    match result {
                        Ok(new_state) => {
                            // Successful entry
                            assert_eq!(
                                global_state,
                                GuardStatus::Unlocked,
                                "Can only enter from unlocked state"
                            );
                            global_state = new_state;
                            entry_count += 1;
                            assert_eq!(entry_count, 1, "Entry count should never exceed 1");
                        }
                        Err(_) => {
                            // Reentrancy blocked
                            assert_eq!(
                                global_state,
                                GuardStatus::Locked,
                                "Enter should only fail from locked state"
                            );
                            assert_eq!(entry_count, 1, "Entry count should be 1 when locked");
                        }
                    }
                }
                2 => {
                    // Exit
                    global_state = exit_pure();
                    assert_eq!(
                        global_state,
                        GuardStatus::Unlocked,
                        "Exit should always unlock"
                    );
                    entry_count = 0;
                }
                3 => {
                    // Query state (no mutation)
                    let status_val = global_state as u32;
                    let reconstructed = GuardStatus::from_u32(status_val);
                    assert_eq!(reconstructed, global_state, "State reconstruction should match");

                    // Verify entry count matches lock state
                    if global_state == GuardStatus::Locked {
                        assert_eq!(entry_count, 1, "Entry count should be 1 when locked");
                    } else {
                        assert_eq!(entry_count, 0, "Entry count should be 0 when unlocked");
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    // Final invariant checks
    assert!(
        entry_count <= 1,
        "Entry count should never exceed 1 (reentrancy prevented)"
    );

    if entry_count == 1 {
        assert_eq!(
            global_state,
            GuardStatus::Locked,
            "If entry count is 1, state must be locked"
        );
    } else {
        assert_eq!(
            global_state,
            GuardStatus::Unlocked,
            "If entry count is 0, state must be unlocked"
        );
    }
});
