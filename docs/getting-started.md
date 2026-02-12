# Getting Started with Sanctifier 🛡️

Sanctifier provides a suite of tools for securing Stellar Soroban smart contracts through static analysis and formal verification.

## Prerequisites

Before installing Sanctifier, ensure you have the following installed:

1.  **Rust**: The industry standard for Soroban development.
    *   Install via [rustup.rs](https://rustup.rs/).
2.  **Soroban CLI**: Required for compiling and interacting with Stellar smart contracts.
    *   Installation: `cargo install --locked soroban-cli`
3.  **Target `wasm32-unknown-unknown`**: Rust target for Wasm.
    *   Command: `rustup target add wasm32-unknown-unknown`

## Installation

Sanctifier is currently distributed as source code within its monorepo. To install the CLI tool locally:

1.  Clone the repository:
    ```bash
    git clone https://github.com/Chidwan3578/Sanctifier.git
    cd Sanctifier
    ```
2.  Install the CLI:
    ```bash
    cargo install --path tooling/sanctifier-cli
    ```

## Running Your First Scan

Once installed, you can analyze any Soroban project by navigating to its root directory (where `Cargo.toml` is located) and running:

```bash
sanctifier analyze
```

### Advanced Usage

You can also specify a path to a contract or choose a different output format:

```bash
# Analyze a specific directory
sanctifier analyze ./contracts/my-vulnerable-contract

# Output as JSON for CI/CD integration
sanctifier analyze --format json
```

## Interpreting the Output

When you run a scan, Sanctifier evaluates your code against several security modules:

### 1. Auth Gaps (Authorization) 🔑
Ensures that all state-modifying or privileged functions include the `require_auth()` check. Missing checks can allow unauthorized users to drain funds or corrupt state.

### 2. Storage Collisions 📦
Analyzes storage keys used in `env.storage().instance()`, `persistent()`, and `temporary()`. It flags potential collisions where different data structures might overwrite each other due to identical key derivation.

### 3. Resource Exhaustion (Gas Usage) ⚡
Estimates the instruction count and resource consumption of your functions. This helps prevent "Out of Gas" errors during runtime and ensures your contract is optimized for the network's limits.

---

For more detailed information on specific vulnerabilities, see the documentation in the `docs/` folder.
