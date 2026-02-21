# Getting Started with Sanctifier

Welcome to Sanctifier! This guide will help you set up the security suite and run your first static analysis scan on your Stellar Soroban smart contracts.

## 1. Prerequisites

Before installing Sanctifier, ensure your development environment is set up with the following:

- **Rust**: Sanctifier is built in Rust. Install [Rust and Cargo](https://rustup.rs/).
- **Soroban CLI**: Required for interacting with Stellar's smart contract platform. Follow the official [Soroban installation guide](https://soroban.stellar.org/docs/getting-started/setup).

## 2. Installation

You can install the Sanctifier CLI directly from the source repository. Navigate to the root directory of the Sanctifier project and run:

```bash
cargo install --path tooling/sanctifier-cli
```

*Note: Make sure your `~/.cargo/bin` directory is in your system's `PATH` to use the `sanctifier` command globally.*

## 3. Running Your First Scan

Once installed, you can analyze your Soroban project to detect potential vulnerabilities or inefficiencies.

1. Navigate to your Soroban contract project directory:

   ```bash
   cd my-soroban-project
   ```

2. Run the Sanctifier analysis suite:

   ```bash
   sanctifier analyze ./contracts/my-token
   ```
   *(Replace `./contracts/my-token` with the actual path to your contract's source code).*

Sanctifier will parse your Rust code and begin its static analysis checks.

## 4. Interpreting the Output

When the scan completes, Sanctifier provides a detailed report. Here is how to interpret the primary checks it performs:

### 🔴 Authorization Gaps
This check ensures that any function modifying state (like transferring tokens or updating admin roles) includes the `require_auth` or `require_auth_for_args` call.
- **Fail**: A state-modifying function was found without proper authorization checks. This is a critical vulnerability.
- **Pass**: All privileged functions correctly authenticate the caller.

### 🟡 Storage Collisions
Sanctifier analyzes your `Instance`, `Persistent`, and `Temporary` storage keys.
- **Fail**: Identical or easily corruptible keys resolve to the same storage slot, potentially overwriting critical data.
- **Pass**: Your contract's data structures safely utilize the storage namespaces without overlap.

### 🔵 Resource Exhaustion (Gas Usage)
This analysis estimates the instruction counts and memory limits required by your contract.
- **Warning**: Functions that loop over dynamic data or perform heavyweight cryptography might exceed Stellar's gas limits (Out-of-Gas), causing transactions to revert.
- **Pass**: Your functions are optimized and safely within network execution limits.
