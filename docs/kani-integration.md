# Kani Rust Verifier Integration with Soroban

This document outlines the limitations and challenges encountered when integrating Kani Rust Verifier with Soroban smart contracts, and describes the proof-of-concept harness for a standard token contract.

## Overview

Kani is a formal verification tool for Rust that can prove properties about code by symbolically executing it. It works best on pure Rust code with minimal external dependencies or FFI calls.

## Limitations with Soroban SDK Host Types

### 1. Host Functions and FFI

Soroban contracts interact with the blockchain environment via `soroban_sdk::Env`. The `Env` trait methods (storage, events, crypto, auth) eventually call `extern "C"` functions provided by the host. Kani **cannot verify** these calls because:

- They are external functions with no implementation visible during contract compilation
- Kani requires source code or a verified model of all called code to reason symbolically
- The host ABI is defined at the Stellar VM layer, not in Rust

### 2. Host Types (Val, Object, Symbol, Address)

| Type    | Limitation for Kani                                                                 |
|---------|-------------------------------------------------------------------------------------|
| `Env`   | Opaque handle; all operations (storage get/set, `require_auth`, events) are FFI     |
| `Val`   | Runtime value type; conversion and comparison require host calls                    |
| `Symbol`| Opaque (u32) handle; `symbol_short!`, comparison, and display need host             |
| `Address`| Opaque handle; `require_auth`, comparison, and display require host                |
| `String`, `Bytes`, `Map`, `Vec` | Host-backed types; operations delegate to env FFI                   |

**Implications:**

- Logic that depends on the *content* of these types cannot be verified without a full host model
- Treating them as integers (e.g. u64) loses semantic meaning and host-enforced invariants
- Authentication flows (`require_auth`) cannot be modelled by Kani

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

### Example: Standard Token Contract

In `contracts/kani-poc/src/lib.rs`, we extract pure balance logic from a standard Soroban token:

```rust
// Verified with Kani — pure primitives only
pub fn transfer_pure(balance_from: i128, balance_to: i128, amount: i128) -> Result<(i128, i128), &'static str> { ... }
pub fn mint_pure(balance: i128, amount: i128) -> Result<i128, &'static str> { ... }
pub fn burn_pure(balance: i128, amount: i128) -> Result<i128, &'static str> { ... }

// Not verified — Host types, FFI
pub fn set_admin(env: Env, new_admin: Symbol) {
    env.storage().instance().set(...);  // Requires Env, Symbol
}
```

A production contract would keep the `#[contractimpl]` layer thin: load balances from `env.storage()`, call the pure functions, then write back.

## PoC Harnesses

The Kani harnesses in `contracts/kani-poc` prove:

| Harness                         | Property                                                  |
|---------------------------------|-----------------------------------------------------------|
| `verify_transfer_pure_conservation` | Transfer preserves total supply: `new_from + new_to == balance_from + balance_to` |
| `verify_transfer_pure_insufficient_balance` | Transfer fails with `Err` when `balance_from < amount`     |
| `verify_mint_pure`              | Mint correctly adds `amount` to `balance`                 |
| `verify_burn_pure`              | Burn correctly subtracts `amount` from `balance`          |

## Running the PoC

Requires [Kani](https://model-checking.github.io/kani/install-guide.html) to be installed:

```bash
cargo install --locked kani-verifier
cargo kani setup
```

Then run:

```bash
cargo kani --package kani-poc-contract
```
