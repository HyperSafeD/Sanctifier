# Getting Started with Sanctifier

Welcome to **Sanctifier** — the comprehensive security and formal verification suite for [Stellar Soroban](https://soroban.stellar.org/) smart contracts. This guide walks you through scanning your first Soroban contract in under 5 minutes.

> **Recording:** An asciinema walkthrough of this tutorial is available at
> [`docs/assets/getting-started.cast`](./assets/getting-started.cast). Play it with
> `asciinema play docs/assets/getting-started.cast` to see the full terminal session.

---

## 1. Prerequisites

Before installing Sanctifier, make sure the following are present on your system.

### Rust & Cargo

Sanctifier is written in Rust and distributed as a Cargo binary. Install the Rust toolchain via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, restart your shell (or run `source ~/.cargo/env`) and confirm:

```bash
rustc --version   # e.g. rustc 1.78.0
cargo --version   # e.g. cargo 1.78.0
```

You will also need the `wasm32-unknown-unknown` target that Soroban contracts compile to:

```bash
rustup target add wasm32-unknown-unknown
```

### Soroban CLI

The Soroban CLI is Stellar's official developer tool for building, deploying, and inspecting contracts. Install it via Cargo:

```bash
cargo install --locked soroban-cli
```

Verify the installation:

```bash
soroban --version   # e.g. soroban 20.x.x
```

> Full setup instructions are available in the [official Soroban docs](https://soroban.stellar.org/docs/getting-started/setup).

---

## 2. Installing Sanctifier

Install the Sanctifier CLI directly from crates.io:

```bash
cargo install sanctifier-cli
```

> **Note:** Ensure `~/.cargo/bin` is on your `PATH`. If not, add it to your shell profile:
>
> ```bash
> echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
> source ~/.bashrc
> ```

Confirm the installation succeeded:

```bash
sanctifier --version
```

Update to the latest Sanctifier binary at any time:

```bash
cargo install sanctifier-cli --force
```

### Shell Completions

Sanctifier supports shell completions for bash, zsh, fish, powershell, and elvish. Generate completions for your shell:

**Bash:**
```bash
sanctifier completions bash > ~/.local/share/bash-completion/completions/sanctifier
```

**Zsh:**
```bash
sanctifier completions zsh > ~/.zfunc/_sanctifier
# Add to ~/.zshrc: fpath=(~/.zfunc $fpath)
```

**Fish:**
```bash
sanctifier completions fish > ~/.config/fish/completions/sanctifier.fish
```

**PowerShell:**
```powershell
sanctifier completions powershell | Out-String | Invoke-Expression
```

**Elvish:**
```bash
sanctifier completions elvish > ~/.elvish/lib/sanctifier.elv
```

After installing completions, restart your shell or source the completion file.

---

## 3. Create a Minimal Soroban Contract

Create a fresh Cargo project and add the Soroban SDK dependency:

```bash
cargo new --lib my-contract
cd my-contract
```

Replace `Cargo.toml` with:

```toml
[package]
name = "my-contract"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
soroban-sdk = { version = "21", features = ["testutils"] }
```

Replace `src/lib.rs` with this intentionally vulnerable contract — it has three findings
for Sanctifier to catch:

```rust
#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Address};

#[contract]
pub struct Counter;

#[contractimpl]
impl Counter {
    // S001: missing require_auth — anyone can increment the counter
    pub fn increment(env: Env, user: Address, by: u32) -> u32 {
        let key = "count";
        let current: u32 = env.storage().persistent().get(&key).unwrap_or(0);
        // S003: unchecked arithmetic — can overflow
        let next = current + by;
        env.storage().persistent().set(&key, &next);
        next
    }

    // S002: panic! aborts the contract
    pub fn reset(env: Env) {
        panic!("reset is not implemented yet");
    }
}
```

---

## 4. Run `sanctifier analyze`

From inside the `my-contract` directory (created in step 3), run:

```bash
sanctifier analyze ./my-contract
```

Sanctifier will print findings for the three issues intentionally left in the contract:

```
🛑 Found potential Authentication Gaps!
   -> Function `increment` is modifying state without require_auth()

🛑 Found explicit Panics/Unwraps!
   -> Function `reset`: panic! aborts the contract (src/lib.rs:reset)

🔢 Found unchecked Arithmetic Operations!
   -> Function `increment`: Unchecked `+` (src/lib.rs:increment)
```

### Other ways to invoke

| Target | Command |
|--------|---------|
| Entire contract directory | `sanctifier analyze ./my-contract` |
| Single source file | `sanctifier analyze ./my-contract/src/lib.rs` |
| Current directory | `cd my-contract && sanctifier analyze` |
| JSON output (for CI) | `sanctifier analyze ./my-contract --format json` |

---

## 5. Project Configuration (`.sanctify.toml`)

Sanctifier looks for a `.sanctify.toml` file in the target directory and its parents. Running `sanctifier init` in your project root scaffolds a default config:

```bash
sanctifier init
```

This creates `.sanctify.toml` with sensible defaults:

```toml
ignore_paths  = ["target", ".git"]
enabled_rules = ["auth_gaps", "panics", "arithmetic", "ledger_size"]
ledger_limit  = 64000
telemetry     = false
strict_mode   = false

# Optional: define regex-based custom rules
[[custom_rules]]
name    = "no_unsafe_block"
pattern = "unsafe\\s*\\{"
severity = "error"

[[custom_rules]]
name    = "no_mem_forget"
pattern = "std::mem::forget"
severity = "warning"
```

Adjust `enabled_rules` to enable or disable specific checks, and add entries to `[[custom_rules]]` to enforce your own patterns.
If you want to opt in to telemetry, run `sanctifier init --telemetry on` or set `telemetry = true` in `.sanctify.toml`. Telemetry only sends rule IDs, analysis duration, and the sanitized tool version. To point it at your own collector, set `SANCTIFIER_TELEMETRY_URL`.

---

## 6. Interpreting the Output

A typical run produces output similar to the following:

```
✨ Sanctifier: Valid Soroban project found at "./contracts/my-token"
🔍 Analyzing contract at "./contracts/my-token"...
✅ Static analysis complete.

🛑 Found potential Authentication Gaps!
   -> Function `transfer` is modifying state without require_auth()

🛑 Found explicit Panics/Unwraps!
   -> Function `mint`: Using `unwrap` (Location: src/lib.rs:transfer)
   💡 Tip: Prefer returning Result or Error types for better contract safety.

🔢 Found unchecked Arithmetic Operations!
   -> Function `compound_interest`: Unchecked `+` (src/lib.rs:compound_interest)
      💡 Use checked_add() or saturating_add() to prevent overflow.

⚠️  Found Ledger Size Warnings!
   LargeState approaches the ledger entry size limit!
      Estimated size: 68200 bytes (Limit: 64000 bytes)

🔔 Found Event Consistency Issues!
   ⚠️  Function `transfer`: Event "Transfer" emits inconsistent topic counts
   💡  Function `mint`: Topic "token_symbol" is a long string; consider `symbol_short!`

📜 Found Custom Rule Matches!
   -> Rule `no_unsafe_block`: `unsafe { ... }` (Line: 42)

🔄 Upgrade Pattern Analysis
   -> [missing_init] Contract has upgrade mechanism but no init function (src/lib.rs:42)
      💡 Add an init() function to set post-upgrade state safely.
```

### Understanding each finding category

#### 🛑 Authentication Gaps
Functions that write to contract storage must call `require_auth()` or `require_auth_for_args()` to verify the caller is authorized. A missing call here is a **critical vulnerability** — anyone could invoke the function.

**Fix:** Add `env.require_auth(&admin)` (or the appropriate principal) at the top of any privileged function.

#### 🛑 Panics & Unwraps
`panic!`, `unwrap()`, and `expect()` abort the entire transaction with a generic error. In production contracts this makes debugging difficult and can be exploited for denial-of-service.

**Fix:** Replace with `Result`-returning functions and propagate errors using the `?` operator or Soroban's `panic_with_error!` macro.

#### 🔢 Unchecked Arithmetic
Plain `+`, `-`, `*` operators can silently overflow in Rust's release builds on the `wasm32` target, producing incorrect balances or state.

**Fix:** Use `checked_add()`, `checked_sub()`, `checked_mul()`, or their `saturating_*` equivalents.

#### ⚠️ Ledger Size Warnings
Soroban enforces a maximum size for each ledger entry (default network limit: 64 KB). Structs whose estimated serialized size approaches or exceeds this limit will fail to write to persistent storage at runtime.

**Fix:** Break large structs into smaller ledger entries, or move infrequently-accessed fields to separate keys.

#### 🔔 Event Consistency Issues
Two sub-checks run here:

- **Inconsistent schema** — the same event name is published with a different number of topics in different call sites, making off-chain indexing unreliable.
- **Optimizable topic** — a topic uses a long `String` where `symbol_short!` (≤ 9 ASCII bytes) would save gas.

**Fix:** Standardize the topic list for each event name and replace eligible string topics with `symbol_short!("name")`.

#### 📜 Custom Rule Matches
Any pattern listed under `[[custom_rules]]` in your `.sanctify.toml` that matches a line in the source is reported here. These are project-specific policies (e.g. banning `unsafe` blocks or `std::mem::forget`).

**Fix:** Review the matched line and refactor to comply with your project's coding standards.

#### 🔄 Upgrade Pattern Analysis
Sanctifier checks for upgrade-related patterns (e.g. `Wasm::upgrade`, missing `init` functions, missing access control on upgrade entry points).

**Fix:** Ensure your upgrade function is admin-gated and that a corresponding `init()` function is present to safely migrate state after an upgrade.

---

## 7. Fix a Finding

Let's address the three findings one by one. Open `my-contract/src/lib.rs` and apply
these changes:

**Fix S001 — add `require_auth`:**

```rust
pub fn increment(env: Env, user: Address, by: u32) -> u32 {
    user.require_auth();                    // <- add this line
    let key = "count";
    let current: u32 = env.storage().persistent().get(&key).unwrap_or(0);
    let next = current.checked_add(by).expect("overflow"); // <- fix S003 too
    env.storage().persistent().set(&key, &next);
    next
}
```

**Fix S002 — remove `panic!` and return a structured error:**

```rust
pub fn reset(_env: Env) -> Result<(), &'static str> {
    Err("reset is not yet supported")
}
```

After applying these changes the file should look like this:

```rust
#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Address};

#[contract]
pub struct Counter;

#[contractimpl]
impl Counter {
    pub fn increment(env: Env, user: Address, by: u32) -> u32 {
        user.require_auth();
        let key = "count";
        let current: u32 = env.storage().persistent().get(&key).unwrap_or(0);
        let next = current.checked_add(by).expect("overflow");
        env.storage().persistent().set(&key, &next);
        next
    }

    pub fn reset(_env: Env) -> Result<(), &'static str> {
        Err("reset is not yet supported")
    }
}
```

---

## 8. Re-run and Confirm Clean

Run Sanctifier again on the fixed contract:

```bash
sanctifier analyze ./my-contract
```

This time the output should show no findings:

```
✨ Sanctifier: Valid Soroban project found at "./my-contract"
🔍 Analyzing contract at "./my-contract"...
✅ Static analysis complete.

No findings — your contract looks clean!
```

Congratulations! You have installed Sanctifier, written a Soroban contract with known
vulnerabilities, interpreted the findings, applied fixes, and confirmed a clean report.

---

## 9. Next Steps

- **Formal Verification** — See [`docs/kani-integration.md`](./kani-integration.md) to add model-checking with the Kani verifier.
- **Runtime Guards** — See [`docs/runtime-guards-integration.md`](./runtime-guards-integration.md) to add runtime invariant wrappers in your existing Soroban contract.
- **Video Tutorials** — See [`docs/formal-verification-video-series.md`](./formal-verification-video-series.md) for short walkthrough episodes on report reading and Kani proofs.
- **CI Integration** — Use `--format json` and pipe the output to your pipeline's static analysis step to fail builds on new findings.
- **Contributing** — Bug reports and new rule ideas are welcome. See `CONTRIBUTING.md` for guidelines.
