# Getting Started with Sanctifier

Welcome to **Sanctifier** — the comprehensive security and formal verification suite for [Stellar Soroban](https://soroban.stellar.org/) smart contracts. This guide walks you through everything you need to go from zero to running your first security scan.

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

Clone the Sanctifier repository and install the CLI from source:

```bash
git clone https://github.com/your-org/sanctifier.git
cd sanctifier
cargo install --path tooling/sanctifier-cli
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

---

## 3. Running Your First Scan

### Option A — Analyze an entire contract directory

Point `sanctifier analyze` at the root of any Soroban contract crate (a directory containing `Cargo.toml` with a `soroban-sdk` dependency):

```bash
sanctifier analyze ./contracts/my-token
```

### Option B — Analyze a single source file

You can also target a specific `.rs` file:

```bash
sanctifier analyze ./contracts/my-token/src/lib.rs
```

### Option C — Analyze the current directory

Running `sanctifier analyze` with no arguments defaults to `.`:

```bash
cd my-soroban-project
sanctifier analyze
```

### Optional flags

| Flag | Description | Default |
|---|---|---|
| `-f`, `--format` | Output format: `text` or `json` | `text` |
| `-l`, `--limit` | Ledger entry size limit in bytes | `64000` |

**JSON output** — useful for CI pipelines and tooling integrations:

```bash
sanctifier analyze ./contracts/my-token --format json
```

---

## 4. Project Configuration (`.sanctify.toml`)

Sanctifier looks for a `.sanctify.toml` file in the target directory and its parents. Running `sanctifier init` in your project root scaffolds a default config:

```bash
sanctifier init
```

This creates `.sanctify.toml` with sensible defaults:

```toml
ignore_paths  = ["target", ".git"]
enabled_rules = ["auth_gaps", "panics", "arithmetic", "ledger_size"]
ledger_limit  = 64000
strict_mode   = false

# Optional: define regex-based custom rules
[[custom_rules]]
name    = "no_unsafe_block"
pattern = "unsafe\\s*\\{"

[[custom_rules]]
name    = "no_mem_forget"
pattern = "std::mem::forget"
```

Adjust `enabled_rules` to enable or disable specific checks, and add entries to `[[custom_rules]]` to enforce your own patterns.

---

## 5. Interpreting the Output

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

## 6. Next Steps

- **Formal Verification** — See [`docs/kani-integration.md`](./kani-integration.md) to add model-checking with the Kani verifier.
- **CI Integration** — Use `--format json` and pipe the output to your pipeline's static analysis step to fail builds on new findings.
- **Contributing** — Bug reports and new rule ideas are welcome. See `CONTRIBUTING.md` for guidelines.
