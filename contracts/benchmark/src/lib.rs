//! Benchmark harness for Sanctifier contracts.
//!
//! # Module boundaries
//!
//! | Module       | Responsibility                                      |
//! |--------------|-----------------------------------------------------|
//! | [`budgets`]  | Compile-time WASM-size and CPU-instruction ceilings |
//! | `token`      | Benchmark tests for the SEP-41 token contract       |
//! | `vesting`    | Benchmark tests for the vesting contract            |
//!
//! Each benchmark test passes if – and only if – the measured operation
//! completes without exhausting the Soroban host's default resource budget.
//! Budget ceilings are documented in [`budgets`] for human reference and
//! enforced at WASM-binary level by `scripts/build-contracts.sh`.

pub mod budgets;

#[cfg(test)]
mod token;
#[cfg(test)]
mod vesting;
