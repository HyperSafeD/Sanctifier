//! Compile-time performance budgets for Sanctifier contracts.
//!
//! ## WASM binary sizes
//!
//! [`WASM_SIZE_BUDGETS`] maps each contract's crate name to its maximum
//! allowed optimised WASM binary size in bytes.  The values are checked by
//! `scripts/build-contracts.sh`; a CI failure is raised when a contract
//! exceeds its budget.
//!
//! ## CPU instruction ceilings
//!
//! The constants below document the *expected* upper bound on Soroban CPU
//! instructions for common operations.  The Soroban host enforces its own
//! hard cap automatically; these constants serve as human-readable baselines
//! that make regressions visible before they hit the hard cap.
//!
//! Adjust a constant (with a justification comment) when accepting a
//! deliberate size or performance trade-off.

/// (contract-crate-name, max-wasm-bytes) pairs checked by the build script.
///
/// Limits are intentionally conservative; tighten them as contracts mature.
pub const WASM_SIZE_BUDGETS: &[(&str, u64)] = &[
    ("amm-pool", 65_536),              // 64 KiB — complex AMM logic
    ("flashloan-token", 32_768),       // 32 KiB
    ("governance-contract", 65_536),   // 64 KiB — proposal + voting state
    ("kani-poc-contract", 16_384),     // 16 KiB — minimal proof-of-concept
    ("multisig-wallet", 65_536),       // 64 KiB
    ("my-contract", 32_768),           // 32 KiB — SEP-41 token
    ("uups-proxy", 32_768),            // 32 KiB
    ("reentrancy-guard", 16_384),      // 16 KiB — guard only
    ("runtime-guard-wrapper", 65_536), // 64 KiB
    ("timelock", 32_768),              // 32 KiB
    ("token-with-bugs", 32_768),       // 32 KiB
    ("vesting-contract", 32_768),      // 32 KiB
    ("vulnerable-contract", 32_768),   // 32 KiB
];

// ---------------------------------------------------------------------------
// SEP-41 token operation ceilings (my-contract)
// ---------------------------------------------------------------------------

/// Max Soroban CPU instructions for `Token::initialize`.
pub const TOKEN_INITIALIZE_CPU: u64 = 3_000_000;

/// Max Soroban CPU instructions for `Token::mint`.
pub const TOKEN_MINT_CPU: u64 = 2_000_000;

/// Max Soroban CPU instructions for `Token::transfer`.
pub const TOKEN_TRANSFER_CPU: u64 = 5_000_000;

/// Max Soroban CPU instructions for `Token::approve`.
pub const TOKEN_APPROVE_CPU: u64 = 3_000_000;

/// Max Soroban CPU instructions for `Token::transfer_from`.
pub const TOKEN_TRANSFER_FROM_CPU: u64 = 7_000_000;

/// Max Soroban CPU instructions for `Token::burn`.
pub const TOKEN_BURN_CPU: u64 = 3_000_000;

// ---------------------------------------------------------------------------
// Vesting operation ceilings
// ---------------------------------------------------------------------------

/// Max Soroban CPU instructions for `VestingContract::init`.
pub const VESTING_INIT_CPU: u64 = 8_000_000;

/// Max Soroban CPU instructions for `VestingContract::vested_amount`.
pub const VESTING_AMOUNT_CPU: u64 = 2_000_000;

/// Max Soroban CPU instructions for `VestingContract::claim`.
pub const VESTING_CLAIM_CPU: u64 = 6_000_000;
