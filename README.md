<div align="center">
  <img src="branding/logo.png" width="220" alt="Sanctifier" />

  # Sanctifier

  ### Catch the bug before someone else cashes it.

  **Security copilot for Stellar Soroban smart contracts** — static analysis, formal verification with Z3, on-chain runtime guards, and an auditor-friendly dashboard, all driven by a single SARIF-clean engine.

  [![CI](https://github.com/HyperSafeD/Sanctifier/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/HyperSafeD/Sanctifier/actions/workflows/ci.yml)
  [![Codecov](https://codecov.io/gh/HyperSafeD/Sanctifier/graph/badge.svg)](https://codecov.io/gh/HyperSafeD/Sanctifier)
  [![crates.io](https://img.shields.io/crates/v/sanctifier-cli.svg)](https://crates.io/crates/sanctifier-cli)
  [![Soroban Testnet](https://img.shields.io/badge/Soroban%20Testnet-Live-2dd4bf?style=flat-square&logo=stellar)](LIVE_TESTNET.md)
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
</div>

---

## Why Sanctifier exists

When an EVM contract ships a bug, the community has a decade of tools — Slither, Mythril, Foundry, Certora — to catch it. Soroban shipped to mainnet in 2024 with almost none of that scaffolding. Every team writes the same review checklist from scratch. Every audit re-discovers the same five footguns.

Sanctifier is the missing layer. **One engine, twelve canonical rules, three deployment surfaces.** Built specifically for Soroban's authorization model, storage TTL semantics, SEP-41 token interface, and gas/event quirks. Open source. Auditor-grade. Drop-in for CI.

---

## What it catches

Every finding has a stable code — `S001..S012` — so you can filter, suppress, and trend it across releases.

| Code | What it catches | Why it bites |
|------|-----------------|--------------|
| `S001` | Missing `require_auth` on state-changing calls | Anyone can drain your contract |
| `S002` | `panic!` / `unwrap` / `expect` in contract paths | Locked state, no recovery |
| `S003` | Unchecked arithmetic — overflow, underflow, truncation | Silent loss-of-funds rounding |
| `S004` | Ledger entries pushing the size threshold | Refusal at write time, mid-tx |
| `S005` | Storage-key collisions between data paths | Cross-feature data corruption |
| `S006` | Unsafe patterns — including timestamp-as-randomness | Predictable winners, exploit replay |
| `S007` | Your custom YAML rules | Your house style, enforced |
| `S008` | Inconsistent or missing event emissions | Wallets and indexers go blind |
| `S009` | Unhandled `Result` return values | Silent failures masquerading as success |
| `S010` | Upgrade / admin / governance risk | Single-key takeover paths |
| `S011` | Z3-disproved invariants | Mathematical guarantees you don't have |
| `S012` | SEP-41 token interface deviations | Wallets reject your token |

Plus the community **vulnerability database** matches known CVE-style patterns (`SOL-2024-*`) against your AST — so a published exploit anywhere becomes a finding everywhere.

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

```bash
# 1. install
cargo install sanctifier-cli

# 2. scan
sanctifier analyze ./contracts/my-token

# 3. ship a badge for your README
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

| Method | Command |
|--------|---------|
| **crates.io** | `cargo install sanctifier-cli` |
| **From source** | `git clone https://github.com/HyperSafeD/Sanctifier && cd Sanctifier && make release` |
| **Codespaces** | [![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/HyperSafeD/Sanctifier) |
| **Docker** | `docker run --rm -v $PWD:/src ghcr.io/hypersafed/sanctifier analyze /src` |

**Prerequisites:** Rust 1.78+, plus `libz3-dev` and `clang`/`libclang-dev` for the Z3 formal-verification backend.

```bash
# Debian/Ubuntu
sudo apt-get install libz3-dev clang libclang-dev
# macOS
brew install z3 llvm
```

Skip Z3 entirely with `cargo install sanctifier-cli --no-default-features` — every rule except `S011` still runs.

---

## CLI reference

```bash
sanctifier analyze    [PATH] [--format text|json] [--limit BYTES] [--webhook-url URL]...
sanctifier diff       [PATH] --baseline <report.json>
sanctifier watch      [PATH]              # re-runs on file change
sanctifier workspace  [PATH]              # cargo-workspace–aware scan
sanctifier callgraph  [PATH] --output callgraph.dot
sanctifier badge      --report report.json --svg-output sanctifier.svg
sanctifier fix        [PATH] --rule S003  # apply patcher fixes
sanctifier verify     [PATH]              # Z3-only invariant pass
sanctifier deploy     ...                 # ship the runtime guard
sanctifier doctor                         # environment diagnostics
sanctifier init                           # generate .sanctify.toml
sanctifier update                         # self-update with checksum check
```

Every subcommand respects `--format json` for machine consumption.

---

## Output is a contract, not a vibe

`--format json` output validates against [`schemas/analysis-output.json`](schemas/analysis-output.json) (JSON Schema draft-07). Every report carries a `schema_version` that bumps independently of the CLI version, so downstream tooling can pin to a schema without coupling to a release cadence.

```jsonc
{
  "metadata":       { "version": "0.1.0", "format": "sanctifier-ci-v1", "timestamp": "…" },
  "summary":        { "critical": 0, "high": 0, "medium": 2, "low": 0 },
  "findings":       { "auth_gaps": [...], "arithmetic_issues": [...], "storage_collisions": [...] },
  "vuln_db_matches": [{ "id": "SOL-2024-002", "severity": "CRITICAL", "matched_at": "…" }],
  "schema_version": "1.0.0"
}
```

SARIF 2.1.0 output is canonical for GitHub code-scanning and any SAST aggregator.

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

**Shipped**
- 12 canonical analysis rules (S001–S012) with stable codes
- CLI, GitHub Action, Web Dashboard, VS Code extension, WASM build
- Live testnet runtime-guard contracts emitting on-chain audit events
- SARIF + JSON output, draft-07 schema, badge generator
- Diff mode, watch mode, cargo-workspace scan, patcher

**In flight** (see the [contrib-wave issues](https://github.com/HyperSafeD/Sanctifier/issues?q=contrib-wave+in%3Atitle))
- Real-LLM provider for `/api/ai/explain` (currently stubbed)
- Editor-agnostic `sanctifier lsp` for Neovim / Helix / Zed
- Streaming `--ndjson` output for incremental piping
- GitHub PR comment formatter with delta vs base
- 20+ new engine rules (allowance race, TTL bumps, cross-contract `try_call`, taint through destructures, …)

**Wishlist**
- Hosted REST API, Stellar Laboratory plugin, cargo-sanctify subcommand shim, anomaly-detection rules engine for recorded runtime calls

---

## Project layout

```text
Sanctifier/
├── tooling/
│   ├── sanctifier-cli/        # CLI binary (the one you install)
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

## Documentation

| If you want to… | Read |
|-----------------|------|
| Get going in 10 minutes | [docs/getting-started.md](docs/getting-started.md) |
| Understand every finding code | [docs/error-codes.md](docs/error-codes.md) |
| Wire the runtime guard into your contract | [docs/runtime-guards-integration.md](docs/runtime-guards-integration.md) |
| Set up CI | [docs/ci-cd-setup.md](docs/ci-cd-setup.md) |
| Deploy to testnet | [docs/soroban-deployment.md](docs/soroban-deployment.md) |
| Write your own rule | [docs/rule-authoring-guide.md](docs/rule-authoring-guide.md) |
| See it benchmarked | [docs/case-studies/soroban-examples.md](docs/case-studies/soroban-examples.md) |
| Review the threat model | [docs/security-threat-model.md](docs/security-threat-model.md) |
| Browse design decisions | [docs/adr/](docs/adr/) |

---

## Contributing

We're picking up momentum and we want the help. **~100 hand-curated [`[contrib-wave]`](https://github.com/HyperSafeD/Sanctifier/issues?q=contrib-wave+in%3Atitle) issues** are live, each one with a problem statement, acceptance criteria, file pointers, and difficulty hint. There's a `good first issue` for every skill level — bash, Rust, TypeScript, Next.js, GitHub Actions, doc-writing, contract authoring.

Start with [CONTRIBUTING.md](CONTRIBUTING.md), then pick an issue and say hi.

---

## License

MIT — see [LICENSE](LICENSE).

<div align="center">
  <sub>Built for the Stellar Soroban ecosystem · Mainnet doesn't forgive · Audit-grade, in CI.</sub>
</div>
