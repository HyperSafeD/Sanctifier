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

### Installation Methods

| Method | Command | Best for |
|--------|---------|----------|
| **Cargo (recommended)** | `cargo install sanctifier-cli --locked` | Most users; includes Z3 verification |
| **Cargo (no Z3)** | `cargo install sanctifier-cli --locked --no-default-features` | Faster install; all rules except S011 work |
| **Docker** | `docker run --rm -v $PWD:/src ghcr.io/hypersafed/sanctifier analyze /src` | No local Rust toolchain needed |
| **Pre-built binary** | Download from [Releases](https://github.com/HyperSafeD/Sanctifier/releases/latest) | No Rust toolchain required |

### Install with Cargo

Install the Sanctifier CLI directly from crates.io:

```bash
cargo install sanctifier-cli --locked
```

> **Note:** Ensure `~/.cargo/bin` is on your `PATH`. If not, add it to your shell profile:
>
> ```bash
> echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
> source ~/.bashrc
> ```

Verify the installation succeeded:

```bash
sanctifier --version
```

Update to the latest Sanctifier binary at any time:

```bash
cargo install sanctifier-cli --locked --force
```

### Pre-built Binaries

Direct downloads for your platform (no Rust toolchain required):

| Platform | Download |
|----------|----------|
| **Linux (x86_64)** | [sanctifier-linux-amd64](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-linux-amd64) |
| **Linux (musl)** | [sanctifier-linux-amd64-musl](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-linux-amd64-musl) |
| **macOS (Intel)** | [sanctifier-macos-amd64](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-macos-amd64) |
| **macOS (Apple Silicon)** | [sanctifier-macos-arm64](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-macos-arm64) |
| **Windows** | [sanctifier-windows-amd64.exe](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-windows-amd64.exe) |

**Verifying binaries:** Each release includes SHA256 checksums for integrity verification:

```bash
# Linux/macOS
curl -LO https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-linux-amd64
curl -LO https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-linux-amd64.sha256
sha256sum -c sanctifier-linux-amd64.sha256

# Windows (PowerShell)
(Get-FileHash sanctifier-windows-amd64.exe).Hash -eq (Get-Content sanctifier-windows-amd64.exe.sha256)
```

### System Requirements

**Minimum:**
- Rust 1.78+ (if installing via cargo)
- 2GB RAM
- 500MB disk space

**For full features (including Z3 formal verification):**

| Platform | Required Packages |
|----------|------------------|
| **Debian/Ubuntu** | `sudo apt-get install libz3-dev clang libclang-dev build-essential pkg-config` |
| **Fedora/RHEL** | `sudo dnf install z3-devel clang clang-devel` |
| **Arch Linux** | `sudo pacman -S z3 clang` |
| **macOS** | `brew install z3 llvm` |
| **Windows** | Install [Z3](https://github.com/Z3Prover/z3/releases) and [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) |

**Optional:**
- `soroban-cli` for contract deployment features: `cargo install soroban-cli`
- `wasm-pack` for WASM analysis: `cargo install wasm-pack`

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
soroban-sdk = { version = "21.7.0", features = ["testutils"] }
```

Replace `src/lib.rs` with this intentionally vulnerable contract — it has three findings
for Sanctifier to catch:

### Example Contract

Here's a complete working example with clear security issues:

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

### Machine-readable output

For scripting or CI, run with `--format json` instead of the default human-readable
terminal output:

```bash
sanctifier analyze ./my-contract --format json
```

```json
{
  "schema_version": "1.0.0",
  "rule_violations": [
    {
      "file": "src/lib.rs",
      "rule_name": "auth_gaps",
      "severity": "Critical",
      "message": "Function `increment` is modifying state without require_auth()",
      "location": "src/lib.rs:increment",
      "suggestion": "Call `user.require_auth()` before mutating storage."
    },
    {
      "file": "src/lib.rs",
      "rule_name": "panics",
      "severity": "High",
      "message": "panic! aborts the contract (src/lib.rs:reset)",
      "location": "src/lib.rs:reset",
      "suggestion": "Return a Result and propagate errors instead of panicking."
    }
  ],
  "error_codes": ["S001", "S002", "S003", "..."],
  "summary": {
    "total_findings": 2,
    "duration_ms": 84,
    "version": "0.x.y"
  }
}
```

Filter the JSON output with [`jq`](https://jqlang.org/) to show only critical findings:

```bash
sanctifier analyze ./my-contract --format json | jq '.rule_violations[] | select(.severity == "Critical")'
```

A [SARIF](https://sarifweb.azurewebsites.net/) output format is also available for GitHub code scanning:

```bash
sanctifier analyze ./my-contract --format sarif
```

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

## 9. CI Integration

`sanctifier analyze` exits `0` when the run is clean, `1` when findings triggered the
active profile (fail the build on this), and `2` on an unrecoverable error such as a
bad path or unparseable config — so a plain exit-code check is enough to gate a
pipeline without parsing any output at all:

```yaml
# .github/workflows/sanctifier.yml
name: Sanctifier

on: [pull_request]

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Sanctifier
        run: cargo install sanctifier-cli --locked

      - name: Run security scan
        run: sanctifier analyze ./contracts --format json | tee sanctifier-report.json
        # Exits 1 on findings, which fails this step (and the job) automatically.

      - name: Upload report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: sanctifier-report
          path: sanctifier-report.json
```

If you'd rather warn instead of fail on findings during a migration period, capture the
exit code explicitly:

```bash
sanctifier analyze ./contracts --format json > report.json; code=$?
if [ "$code" -eq 2 ]; then
  echo "Sanctifier itself errored — treat as a build failure." >&2
  exit 1
elif [ "$code" -eq 1 ]; then
  echo "::warning::Sanctifier found issues — see report.json"
fi
```

## 10. Troubleshooting

Common errors while working through this guide, and how to resolve them.

### `error: no such command: 'install'` or `cargo: command not found`

Rust/Cargo isn't on your `PATH` yet. Re-run `source ~/.cargo/env` in your current shell (rustup adds
this to your shell profile, but it only takes effect in *new* shells), or open a new terminal. Confirm
with `cargo --version` before retrying.

### `error[E0463]: can't find crate for 'core'` (or similar) when building a contract

The `wasm32-unknown-unknown` target isn't installed. Run:

```bash
rustup target add wasm32-unknown-unknown
```

If you're on a newer Rust toolchain (1.82+) and still see this, some Soroban SDK versions require
`wasm32v1-none` instead — check your `soroban-sdk` version's release notes and, if needed:

```bash
rustup target add wasm32v1-none
```

### `error: failed to run custom build command for 'soroban-sdk'` during `cargo install sanctifier-cli`

This usually means your Rust toolchain is older than what `soroban-sdk` requires. Update Rust first:

```bash
rustup update stable
```

then retry the install. If it still fails, check the exact `soroban-sdk` version Sanctifier depends
on (`cargo tree -p sanctifier-cli | grep soroban-sdk` after a partial install, or check
[`Cargo.toml`](../Cargo.toml)) against your installed toolchain's supported range.

### `sanctifier: command not found` after a successful `cargo install`

`cargo install` places binaries in `~/.cargo/bin`, which needs to be on your `PATH`. This is normally
set up by the rustup shell profile changes from step 1 — if you skipped that, add it manually:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

and add the line to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) so it persists across sessions.

### `No Soroban project found at "..."` when running `sanctifier analyze`

Sanctifier expects to find a `Cargo.toml` with a Soroban contract crate (a dependency on
`soroban-sdk`) at or under the given path. Double-check:

- You're pointing at the contract's directory (or a parent directory containing one), not an
  unrelated path.
- The target `Cargo.toml` actually lists `soroban-sdk` as a dependency — a plain Rust crate without
  it won't be recognized as a Soroban project.
- If you meant to scan a single file, pass the file path directly (`sanctifier analyze
  src/lib.rs`) rather than a directory, per the usage shown in step 3.

### Findings look stale after fixing the reported issue

Sanctifier re-reads the file from disk on every run, so a stale result almost always means the fix
wasn't saved, or you're pointing the CLI at a different path than the one you edited (e.g. a
build artifact or a copy under `target/`). Re-run with the exact file/directory path you edited and
confirm the timestamp on the file matches your edit.

### Z3/Kani formal-verification steps fail with a solver timeout

The default solver timeout is tuned for typical contract sizes; a very large or arithmetically dense
function can exceed it. Findings reported as timeouts (rather than a concrete violation) are not
proof of a bug — they mean the solver couldn't decide either way within the time budget. See
[`docs/kani-integration.md`](./kani-integration.md) for how to raise the timeout or scope a harness
to a smaller pure function (the same "Core Logic Separation" pattern used throughout
`contracts/kani-poc`).

### Still stuck?

Open an issue with your OS, `rustc --version`, `cargo --version`, and the exact command + output —
see `CONTRIBUTING.md` for the issue template.

---

## 11. Next Steps

- **Formal Verification** — See [`docs/kani-integration.md`](./kani-integration.md) to add model-checking with the Kani verifier.
- **Runtime Guards** — See [`docs/runtime-guards-integration.md`](./runtime-guards-integration.md) to add runtime invariant wrappers in your existing Soroban contract.
- **Video Tutorials** — See [`docs/formal-verification-video-series.md`](./formal-verification-video-series.md) for short walkthrough episodes on report reading and Kani proofs.
- **Contributing** — Bug reports and new rule ideas are welcome. See `CONTRIBUTING.md` for guidelines.
