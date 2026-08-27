<div align="center">

**[English](README.md)** | **[Español](README.es.md)** | **[中文](README.zh-CN.md)** | **[日本語](README.ja.md)** | **[Français](README.fr.md)**

</div>

<div align="center">
  <img src="branding/logo.png" width="220" alt="Sanctifier" />

  # Sanctifier

  ### Catch the bug before someone else cashes it.

  **Security copilot for Stellar Soroban smart contracts** — static analysis, formal verification with Z3, on-chain runtime guards, and an auditor-friendly dashboard, all driven by a single SARIF-clean engine.

  [![CI](https://github.com/HyperSafeD/Sanctifier/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/HyperSafeD/Sanctifier/actions/workflows/ci.yml)
  [![Codecov](https://codecov.io/gh/HyperSafeD/Sanctifier/graph/badge.svg)](https://codecov.io/gh/HyperSafeD/Sanctifier)
  [![crates.io](https://img.shields.io/crates/v/sanctifier-cli.svg)](https://crates.io/crates/sanctifier-cli)
  [![Soroban Testnet](https://img.shields.io/badge/Soroban%20Testnet-Live-2dd4bf?style=flat-square&logo=stellar)](LIVE_TESTNET.md)
  [![Testnet Monitor](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/HyperSafeD/Sanctifier/monitor/badges/testnet-monitor.json)](LIVE_TESTNET.md)
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
</div>

---

## Why Sanctifier exists

> [!NOTE]
> When an EVM contract ships a bug, the community has a decade of tools — Slither, Mythril, Foundry, Certora — to catch it. Soroban shipped to mainnet in 2024 with almost none of that scaffolding. Every team writes the same review checklist from scratch. Every audit re-discovers the same five footguns.

Sanctifier is the missing layer. **One engine, twelve canonical rules, three deployment surfaces.** Built specifically for Soroban's authorization model, storage TTL semantics, SEP-41 token interface, and gas/event quirks. Open source. Auditor-grade. Drop-in for CI.

---

## What it catches

Every finding has a stable code — `S001..S012` — so you can filter, suppress, and trend it across releases.

| Code | What it catches | Why it bites |
|------|-----------------|--------------|
| [`S001`](docs/rules/S001.md) | Missing `require_auth` on state-changing calls | Anyone can drain your contract |
| [`S002`](docs/rules/S002.md) | `panic!` / `unwrap` / `expect` in contract paths | Locked state, no recovery |
| [`S003`](docs/rules/S003.md) | Unchecked arithmetic — overflow, underflow, truncation | Silent loss-of-funds rounding |
| [`S004`](docs/rules/S004.md) | Ledger entries pushing the size threshold | Refusal at write time, mid-tx |
| [`S005`](docs/rules/S005.md) | Storage-key collisions between data paths | Cross-feature data corruption |
| [`S006`](docs/rules/S006.md) | Unsafe patterns — including timestamp-as-randomness | Predictable winners, exploit replay |
| [`S007`](docs/rules/S007.md) | Your custom YAML rules | Your house style, enforced |
| [`S008`](docs/rules/S008.md) | Inconsistent or missing event emissions | Wallets and indexers go blind |
| [`S009`](docs/rules/S009.md) | Unhandled `Result` return values | Silent failures masquerading as success |
| [`S010`](docs/rules/S010.md) | Upgrade / admin / governance risk | Single-key takeover paths |
| [`S011`](docs/rules/S011.md) | Z3-disproved invariants | Mathematical guarantees you don't have |
| [`S012`](docs/rules/S012.md) | SEP-41 token interface deviations | Wallets reject your token |

Plus the community **vulnerability database** matches known CVE-style patterns (`SOL-2024-*`) against your AST — so a published exploit anywhere becomes a finding everywhere.

### Zero-knowledge contracts — the `Z001..Z014` series

If your contract verifies ZK proofs on-chain, Sanctifier checks the ways those integrations keep breaking: nullifiers that are never recorded, public inputs that don't commit to the transaction, verifying keys with no ceremony provenance or trusted straight out of storage.

> [!IMPORTANT]
> **→ [docs/zk-roadmap.md](docs/zk-roadmap.md)** is the scope summary: which Z-rules have detectors today, which are documented but not yet wired, and what is deliberately deferred. Start there before reading the 62 individual issues.
>
> The full catalogue lives in [docs/rules/](docs/rules/), with the vulnerability classes and secure patterns explained in the [ZK Security Guide](docs/zk-security-guide.md).

---

## Live on Soroban testnet — right now

This isn't a slide deck. Sanctifier's **Runtime Guard Wrapper**, **Reentrancy Guard**, and **Vulnerable-by-design Contract** are deployed and emitting on-chain audit events you can `stellar contract invoke` against today. See **[LIVE_TESTNET.md](LIVE_TESTNET.md)** for addresses, verification commands, and event logs.

```bash
# Tail real-time guard events on the live deployment
stellar events --network testnet --start-ledger <LATEST> \
  --id $RUNTIME_GUARD_CONTRACT_ID
```

---

## Five ways to use it

| Surface | For | Time to first finding |
|---|---|---|
| **`sanctifier` CLI** | Local dev, scripts, hot paths | **30 seconds** |
| **GitHub Action** | Every PR, every push | **One commit** |
| **Web Dashboard** (Next.js) | Auditors, reviewers, hackathon demos | Drag-and-drop a `.rs` file |
| **VS Code Extension** | Inline diagnostics as you type | One install |
| **On-chain Runtime Guard** | Forensic trail after deploy | One contract wrap |

Same engine under all of them (it cross-compiles to WASM for the browser path), so findings are bit-for-bit identical wherever you scan.

---

## 30-second quickstart

> [!TIP]
> **Skip Z3**: You can install without Z3 formal verification by appending `--no-default-features` to the cargo command.

```bash
# 1. install
cargo install sanctifier-cli

# 2. scan
sanctifier analyze ./contracts

# 3. integrate into CI — exit 1 on high/critical findings
sanctifier analyze ./contracts --exit-code --format sarif > sanctifier.sarif

# 4. ship a security badge for your README
sanctifier analyze . --format json > report.json
sanctifier badge --report report.json --svg-output sanctifier.svg
```

<details>
<summary><b>What you'll see</b></summary>

```text
⚠️ Authentication Gaps
   → [S001] src/lib.rs:transfer — missing require_auth
   → [S001] src/lib.rs:mint     — missing require_auth

⚠️ Unchecked Arithmetic
   → [S003] src/lib.rs:transfer:30 — operator `-`
   → [S003] src/lib.rs:transfer:33 — operator `+`

⚠️ SEP-41 Deviation
   → [S012] missing `allowance` function

🛡️ 2 known-vulnerability matches from DB v1.0.0
   ❌ [SOL-2024-002] Missing auth on token transfer (CRITICAL)
   🔴 [SOL-2024-003] Unchecked balance underflow (HIGH)

✨ Scan complete · 4 findings · exit 1
```

Exit code is `1` when critical/high findings are present — wire it into CI as-is.

</details>

---

## Wire it into your repo (in one PR)

```yaml
# .github/workflows/sanctifier.yml
name: Sanctifier
on: [pull_request, push]
permissions: { contents: read, security-events: write }
jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: HyperSafeD/Sanctifier@main
        with:
          path: .
          format: sarif
          min-severity: high
          upload-sarif: "true"
```

SARIF lands in GitHub code-scanning so reviewers see annotations inline on PRs.

> [!TIP]
> Ensure you have `security-events: write` permissions enabled in your GitHub Actions settings for SARIF uploads to succeed.

---

## Run the dashboard locally

```bash
cd frontend
npm install
npm run dev
# → http://localhost:3000
```

- **`/scan`** — drag in a `.rs` file, get findings in <2s
- **`/dashboard`** — load a JSON report, drill in by severity, see a live call-graph
- **`/playground`** — try canned vulnerable contracts (auth-gap, overflow, unsafe-PRNG, …)
- **`/terminal`** — `sanctifier` in a terminal emulator for guided demos

---

## Install options

### Quick Install (Recommended)

The fastest way to install Sanctifier is via cargo from crates.io:

```bash
cargo install sanctifier-cli
```

This installs the latest stable release with all features enabled, including Z3 formal verification.

### Alternative Installation Methods

| Method | Command | Notes |
|--------|---------|-------|
| **crates.io (latest)** | `cargo install sanctifier-cli` | Recommended for most users |
| **crates.io (no Z3)** | `cargo install sanctifier-cli --no-default-features` | Lighter install, skips formal verification |
| **From source** | `git clone https://github.com/HyperSafeD/Sanctifier && cd Sanctifier && make release` | Latest development version |
| **GitHub Codespaces** | [![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/HyperSafeD/Sanctifier) | Pre-configured cloud environment |
| **Docker** | `docker run --rm -v $PWD:/src ghcr.io/hypersafed/sanctifier analyze /src` | No local install needed |

### Pre-built Binaries

Direct downloads for your platform (no Rust toolchain required):

| Platform | Download | Verification |
|----------|----------|--------------|
| **Linux (x86_64)** | [Download](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-linux-amd64) | [SHA256](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-linux-amd64.sha256) |
| **Linux (musl)** | [Download](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-linux-amd64-musl) | [SHA256](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-linux-amd64-musl.sha256) |
| **macOS (Intel)** | [Download](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-macos-amd64) | [SHA256](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-macos-amd64.sha256) |
| **macOS (Apple Silicon)** | [Download](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-macos-arm64) | [SHA256](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-macos-arm64.sha256) |
| **Windows** | [Download](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-windows-amd64.exe) | [SHA256](https://github.com/HyperSafeD/Sanctifier/releases/latest/download/sanctifier-windows-amd64.exe.sha256) |

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

### Lightweight Installation (Skip Z3)

If you don't need formal verification (rule S011), install without Z3 dependencies:

```bash
cargo install sanctifier-cli --no-default-features
```

This reduces installation time and removes the Z3 dependency requirement. All other rules (S001-S010, S012) remain fully functional.

### Verifying Installation

After installation, verify Sanctifier is working:

```bash
# Check version
sanctifier --version

# Run environment diagnostics
sanctifier doctor

# Test with a sample scan
sanctifier analyze --help
```

### Updating Sanctifier

Keep your installation up-to-date:

```bash
# Update via cargo
cargo install sanctifier-cli --force

# Or use built-in updater with integrity checks
sanctifier update
```

### Troubleshooting Installation

> [!WARNING]
> **Common Issues and Fixes**
>
> 1. **"cargo: command not found"**
>    - Install Rust via rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
>    - Restart your terminal or run: `source ~/.cargo/env`
>
> 2. **"failed to compile z3-sys"**
>    - Install Z3 development libraries (see System Requirements above)
>    - Or install without Z3: `cargo install sanctifier-cli --no-default-features`
>
> 3. **"sanctifier: command not found" after installation**
>    - Ensure `~/.cargo/bin` is in your PATH
>    - Add to shell profile: `echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc`
>
> 4. **Windows: "VCRUNTIME140.dll not found"**
>    - Install [Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
>
> For more detailed troubleshooting, see [`docs/getting-started.md#troubleshooting`](docs/getting-started.md#troubleshooting).

---

## CLI reference

```bash
# Full analysis (most flags shown; all have defaults)
sanctifier analyze  [PATH]
    --format text|json|sarif|ndjson   # output format (default: text)
    --limit BYTES                     # ledger entry size cap (default: 64000)
    --timeout SECS                    # per-file timeout, 0 = none (default: 30)
    --exit-code                       # exit 1 when findings meet threshold
    --min-severity critical|high|medium|low  # threshold for --exit-code (default: high)
    --profile strict|lenient|ci|audit # preset overrides --exit-code/--min-severity
    --webhook-url URL                 # POST results here on completion (repeatable)
    --no-cache                        # skip incremental analysis cache

# Other commands
sanctifier diff       [PATH] --baseline <report.json>   # new/resolved findings vs baseline
sanctifier watch      [PATH]              # re-runs on file change
sanctifier workspace  [PATH]              # cargo-workspace-aware scan
sanctifier callgraph  [PATH] --output callgraph.dot
sanctifier harness    [PATH] --output fuzz-harness --target afl|honggfuzz|both
sanctifier badge      --report report.json --svg-output sanctifier.svg
sanctifier fix        [PATH] --rule S003  # apply patcher fixes
sanctifier verify     [PATH]              # Z3-only invariant pass
sanctifier deploy     [PATH] --network testnet|futurenet|mainnet
sanctifier doctor                         # environment diagnostics
sanctifier init       [PATH]              # scaffold project + .sanctify.toml
sanctifier update                         # self-update with checksum check
```

Every subcommand accepts `--format json` for machine consumption. Use `--format ndjson` with `analyze` for streaming line-delimited output (one JSON object per finding, final `{"event":"done"}`).

---

## Output is a contract, not a vibe

`--format json` output validates against [`schemas/analysis-output.json`](schemas/analysis-output.json) (JSON Schema draft-07). Every report carries a `schema_version` that bumps independently of the CLI version, so downstream tooling can pin to a schema without coupling to a release cadence.

```jsonc
{
  "metadata":        { "version": "0.1.0", "format": "sanctifier-ci-v1", "timestamp": "…" },
  "summary":         { "critical": 0, "high": 1, "medium": 2, "low": 0 },
  "error_codes":     ["S001", "S003"],
  "auth_gaps":       [{ "location": "src/lib.rs:42", "message": "missing require_auth" }],
  "arithmetic_issues": [{ "location": "src/lib.rs:30", "operator": "-" }],
  "rule_violations": [{ "rule_name": "require_auth_for_args", "severity": "Error",
                        "location": "src/lib.rs:set_admin", "message": "…" }],
  "vuln_db_matches": [{ "id": "SOL-2024-002", "severity": "CRITICAL", "matched_at": "src/lib.rs:55" }],
  "schema_version":  "1.0.0"
}
```

SARIF 2.1.0 output is canonical for GitHub code-scanning and any SAST aggregator.

> [!NOTE]
> `--format sarif` produces a SARIF 2.1.0 document compatible with GitHub code-scanning.
> `--format ndjson` streams one object per finding so large scans can be processed incrementally.

---

## Config — `.sanctify.toml`

```toml
ignore_paths        = ["target", ".git"]
enabled_rules       = ["auth_gaps", "panics", "arithmetic", "ledger_size"]
ledger_limit        = 64000
approaching_threshold = 0.8
strict_mode         = false

[[custom_rules]]
name     = "no_unsafe_block"
pattern  = 'unsafe\s*\{'
severity = "error"
```

Custom rules support full YAML DSL — see [docs/rule-authoring-guide.md](docs/rule-authoring-guide.md).

---

## Roadmap

Sanctifier is shipping in waves. What's done, what's next, what's wishlist:

### Shipped
- 12 canonical analysis rules (S001–S012) with stable codes
- CLI, GitHub Action, Web Dashboard, VS Code extension, WASM build
- Off-chain anomaly detector for recorded runtime calls with Slack/Discord alerts
- Live testnet runtime-guard contracts emitting on-chain audit events
- SARIF + JSON output, draft-07 schema, badge generator
- Diff mode, watch mode, cargo-workspace scan, patcher

### In flight (see the [contrib-wave issues](https://github.com/HyperSafeD/Sanctifier/issues?q=contrib-wave+in%3Atitle))
- Real-LLM provider for `/api/ai/explain` (currently stubbed)
- Editor-agnostic `sanctifier lsp` for Neovim / Helix / Zed
- Streaming `--ndjson` output for incremental piping
- GitHub PR comment formatter with delta vs base
- 20+ new engine rules (allowance race, TTL bumps, cross-contract `try_call`, taint through destructures, …)
- ZK integration — `Z001..Z014` rule catalogue, circom/Noir parsing, shielded-contract fixtures, dashboard ZK panel. Scope and status: **[docs/zk-roadmap.md](docs/zk-roadmap.md)**

### Wishlist
- Hosted REST API, Stellar Laboratory plugin, cargo-sanctify subcommand shim, anomaly-detection rules engine for recorded runtime calls

---

## Project layout

```text
Sanctifier/
├── tooling/
│   ├── sanctifier-cli/        # CLI binary (the one you install)
│   ├── sanctifier-detector/   # Off-chain anomaly detection service
│   ├── sanctifier-core/       # Static-analysis engine + Z3 backend
│   └── sanctifier-wasm/       # Browser/Node WASM build of the engine
├── frontend/                  # Next.js dashboard, playground, terminal
├── vscode-extension/          # VS Code diagnostics integration
├── contracts/                 # Soroban contracts (fixtures + live targets)
│   ├── runtime-guard-wrapper/ # ← deployed to testnet
│   ├── reentrancy-guard/      # ← deployed to testnet
│   └── vulnerable-contract/   # ← deployed to testnet (demo target)
├── schemas/
│   └── analysis-output.json   # JSON Schema (draft-07) — validated in CI
├── data/
│   └── vulnerability-db.json  # Community-sourced CVE-style patterns
├── action.yml                 # GitHub composite action
├── benchmarks/                # Performance corpora
├── specs/                     # OpenAPI + RFC drafts
└── docs/                      # Guides, ADRs, threat models, case studies
```

---

## New here? Start with the tutorial

**[Scan your first Soroban contract in 5 minutes →](docs/getting-started.md)**

The tutorial walks you through installing Sanctifier, writing a minimal contract,
running your first scan, fixing every finding, and confirming a clean report — all
in a single terminal session.

---

## Documentation

| If you want to… | Read |
|-----------------|------|
| **Get started (tutorial)** | **[docs/getting-started.md](docs/getting-started.md)** |
| Browse the API reference | [API Documentation](https://hypersafed.github.io/Sanctifier/) |
| Understand every finding code | [docs/error-codes.md](docs/error-codes.md) |
| **Analyse a ZK contract** | **[docs/zk-roadmap.md](docs/zk-roadmap.md)** (scope) · [ZK Security Guide](docs/zk-security-guide.md) · [ZK Integration Guide](docs/ZK-INTEGRATION-GUIDE.md) |
| Wire the runtime guard into your contract | [docs/runtime-guards-integration.md](docs/runtime-guards-integration.md) |
| Set up CI | [docs/ci-cd-setup.md](docs/ci-cd-setup.md) |
| Deploy to testnet | [docs/soroban-deployment.md](docs/soroban-deployment.md) |
| Write your own rule | [docs/rule-authoring-guide.md](docs/rule-authoring-guide.md) |
| See it benchmarked | [docs/case-studies/soroban-examples.md](docs/case-studies/soroban-examples.md) |
| Review the threat model | [docs/security-threat-model.md](docs/security-threat-model.md) |
| **Check service reliability targets** | **[docs/SLO.md](docs/SLO.md)** — uptime, latency, and error budgets for the hosted API |
| Rollback procedures for mainnet | [ROLLBACK_PROCEDURE.md](./ROLLBACK_PROCEDURE.md) |
| Understand versioning policy | [VERSIONING_POLICY.md](./VERSIONING_POLICY.md) |
| Browse design decisions | [docs/adr/](docs/adr/) |

---

## Contributing

> [!TIP]
> We're picking up momentum and we want the help. **~100 hand-curated [`[contrib-wave]`](https://github.com/HyperSafeD/Sanctifier/issues?q=contrib-wave+in%3Atitle) issues** are live, each one with a problem statement, acceptance criteria, file pointers, and difficulty hint.
> 
> There's a `good first issue` for every skill level — bash, Rust, TypeScript, Next.js, GitHub Actions, doc-writing, contract authoring. Start with [CONTRIBUTING.md](CONTRIBUTING.md), then pick an issue and say hi.

---

## License

MIT — see [LICENSE](LICENSE).

<div align="center">
  <sub>Built for the Stellar Soroban ecosystem · Mainnet doesn't forgive · Audit-grade, in CI.</sub>
</div>
