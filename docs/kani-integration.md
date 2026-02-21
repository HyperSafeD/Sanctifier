# Kani Rust Verifier Integration with Soroban

This document outlines the limitations and challenges encountered when integrating Kani Rust Verifier with Soroban smart contracts.

## Overview

Kani is a formal verification tool for Rust that can prove properties about code by symbolically executing it. It works best on pure Rust code with minimal external dependencies or FFI calls.

## Limitations with Soroban SDK

### 1. Host Functions and FFI
Soroban contracts interact with the blockchain environment via `soroban_sdk::Env`. The `Env` trait methods (like storage, events, crypto) eventually call `extern "C"` functions provided by the host environment. Kani cannot verify these calls directly because:
- They are external functions without a visible implementation during compilation of the contract crate.
- Kani requires the source code or a verified model of all dependencies to reason about them.

### 2. Host Types (Val, Object, Symbol)
Soroban SDK types like `Val`, `Object`, `Symbol`, and `Address` are often opaque handles (u64 wrappers) that have meaning only within the context of the host environment. Operations on these types (e.g., converting a `Symbol` to a string, checking an `Address`) require host calls.
- Verifying logic that depends on the *content* of these types is difficult without a full host implementation.
- Symbolic execution of these types as simple integers (u64) is possible but doesn't capture their semantic meaning or constraints enforced by the host.

### 3. `soroban-env-host` Complexity
While `soroban-env-host` provides a Rust implementation of the host environment (used for local testing), verifying contracts linked against it is challenging:
- **Complexity**: The host environment is large and complex, involving memory management, storage emulation, and VM logic.
- **State Explosion**: Symbolic execution of the entire host stack leads to state space explosion, making verification extremely slow or infeasible for non-trivial contracts.
- **Dependencies**: The host environment pulls in many dependencies which increase the verification burden.

## Proof-of-Concept Strategy

To leverage Kani effectively, we recommend a **"Core Logic Separation"** pattern:

1.  **Isolate Logic**: Extract critical business logic into pure Rust functions that do not depend on `soroban_sdk::Env` or host types.
2.  **Verify Pure Functions**: Use Kani to verify these pure functions against properties (e.g., no overflow, invariant preservation).
3.  **Thin Contract Layer**: Keep the actual contract implementation (the `#[contractimpl]` block) as a thin layer that only marshals data between the host environment and the verified pure logic.

### Example

In `contracts/kani-poc/src/lib.rs`, we demonstrate this by verifying a `transfer_pure` function that operates on `i128` balances:

```rust
// Verified with Kani
pub fn transfer_pure(balance_from: i128, balance_to: i128, amount: i128) -> (i128, i128) { ... }

// Not verified (Thin layer)
pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
    // Get balances from storage
    // Call transfer_pure
    // Set new balances to storage
}
```

## Running the PoC

To run the Kani verification on the PoC contract (requires Kani to be installed):

```bash
cargo kani --package kani-poc-contract
```

This will run the harnesses defined in `contracts/kani-poc/src/lib.rs`.
