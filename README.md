# Sanctifier 🛡️

[![CI](https://github.com/HyperSafeD/Sanctifier/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/HyperSafeD/Sanctifier/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/Jayy4rl/Sanctifier/graph/badge.svg)](https://codecov.io/gh/Jayy4rl/Sanctifier)
[![crates.io](https://img.shields.io/crates/v/sanctifier-cli.svg)](https://crates.io/crates/sanctifier-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/HyperSafeD/Sanctifier)

<p align="center">
  <img src="branding/logo.png" width="300" alt="Sanctifier Logo">
</p>

**Sanctifier** is a security and formal-verification suite for
[Stellar Soroban](https://soroban.stellar.org/) smart contracts.
It statically analyses Rust/Soroban source code, checks for 12 classes of
vulnerabilities, matches against a community-sourced vulnerability database,
and optionally proves invariants with Z3.

---

## Table of Content

- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Finding Codes](#-finding-codes)
- [CLI Reference](#-cli-reference)
- [Example JSON Output](#-example-json-output)
- [JSON Schema](#-json-schema)
- [GitHub Action](#-github-action)
- [Project Structure](#-project-structure)
- [Configuration](#-configuration)
- [Add a Sanctifier Badge to Your Project](#-add-a-sanctifier-badge-to-your-project)
- [Documentation](#-documentation)
- [Contributing](#-contributing)
- [License](#-license)

---

## 📦 Installation

### From crates.io

```bash
cargo install sanctifier-cli
```

### From source

```bash
git clone https://github.com/Jayy4rl/Sanctifier.git
cd Sanctifier/tooling/sanctifier-cli
cargo install --path .
```

> **Prerequisites:** Rust 1.78+, `libz3-dev` and `clang`/`libclang-dev`
> (needed by the Z3 formal-verification backend).
>
> ```bash
> # Debian / Ubuntu
> sudo apt-get install libz3-dev clang libclang-dev
>
> # macOS
> brew install z3 llvm
> ```

---

## 🚀 Quick Start

Analyse a Soroban contract in a single command:

```bash
sanctifier analyze ./contracts/my-token
```

<details>
<summary><b>Example terminal output</b></summary>

```text
⚠️ Found potential Authentication Gaps!
   -> [S001] Function: ./contracts/token-with-bugs/src/lib.rs:initialize
   -> [S001] Function: ./contracts/token-with-bugs/src/lib.rs:transfer
   -> [S001] Function: ./contracts/token-with-bugs/src/lib.rs:mint

⚠️ Found unchecked Arithmetic Operations!
   -> [S003] Op: -
      Location: ./contracts/token-with-bugs/src/lib.rs:transfer:30
   -> [S003] Op: +
      Location: ./contracts/token-with-bugs/src/lib.rs:transfer:33

⚠️ Found Unhandled Result issues!
   -> [S009] Function: transfer
      Call: Self :: balance (e . clone () , from . clone ())
      Location: ./contracts/token-with-bugs/src/lib.rs:transfer:27
      Message: Result returned from function call is not handled.

⚠️ Found SEP-41 Interface Deviations!
   -> [S012] Function: allowance
      Kind: MissingFunction
      Message: Missing SEP-41 function 'allowance'.

🛡️ Found 2 known vulnerability pattern(s) (DB v1.0.0)!
   ❌ [SOL-2024-002] Missing Auth on Token Transfer (CRITICAL)
   🔴 [SOL-2024-003] Unchecked Balance Underflow (HIGH)

✨ Static analysis complete.
```

</details>

---

## 🔎 Finding Codes

Every finding is tagged with a stable code so you can filter, suppress, or
reference it in CI.

| Code   | Category            | Description                                                      |
| ------ | ------------------- | ---------------------------------------------------------------- |
| `S001` | authentication      | Missing `require_auth` in a state-changing function              |
| `S002` | panic_handling      | `panic!` / `unwrap` / `expect` usage that may abort execution    |
| `S003` | arithmetic          | Unchecked arithmetic with overflow/underflow risk                |
| `S004` | storage_limits      | Ledger entry size exceeds or approaches the configured threshold |
| `S005` | storage_keys        | Potential storage-key collision across data paths                |
| `S006` | unsafe_patterns     | Potentially unsafe language or runtime pattern detected          |
| `S007` | custom_rule         | User-defined rule matched contract source                        |
| `S008` | events              | Inconsistent topic counts or sub-optimal gas patterns in events  |
| `S009` | logic               | A `Result` return value is not consumed or handled               |
| `S010` | upgrades            | Security risk in contract upgrade or admin mechanisms            |
| `S011` | formal_verification | Z3 proved a mathematical violation of an invariant               |
| `S012` | token_interface     | SEP-41 token interface compatibility or authorization deviation  |

In addition, the community vulnerability database emits `SOL-2024-*` codes
when a known vulnerability pattern is matched.
See [docs/error-codes.md](docs/error-codes.md) for full details.

---

## 🛠 CLI Reference

### `sanctifier analyze`

Analyse a Soroban contract for vulnerabilities.

```bash
sanctifier analyze [OPTIONS] [PATH]
```

| Flag                  | Short | Default  | Description                                             |
| --------------------- | ----- | -------- | ------------------------------------------------------- |
| `[PATH]`              | —     | `.`      | Contract directory or `Cargo.toml`                      |
| `--format <FORMAT>`   | `-f`  | `text`   | Output format: `text` or `json`                         |
| `--limit <BYTES>`     | `-l`  | `64000`  | Ledger entry size limit in bytes                        |
| `--vuln-db <PATH>`    | —     | built-in | Custom vulnerability database JSON                      |
| `--webhook-url <URL>` | —     | —        | Webhook endpoint(s) for scan notifications (repeatable) |

```bash
# JSON output for CI
sanctifier analyze ./contracts/my-token --format json

# Custom ledger limit and webhook
sanctifier analyze . --limit 32000 \
  --webhook-url https://hooks.slack.com/services/XXX/YYY/ZZZ
```

**Exit code:** `1` when critical or high findings are detected (useful in CI
pipelines).

---

### Webhook Integration

Use one or more `--webhook-url` flags to push a `scan.completed` notification
after `sanctifier analyze` finishes:

```bash
sanctifier analyze ./contracts/token-with-bugs \
  --webhook-url https://discord.com/api/webhooks/XXX/YYY \
  --webhook-url https://hooks.slack.com/services/AAA/BBB/CCC
```

Provider-specific payloads:

| Provider       | Payload shape                                                                                       |
| -------------- | --------------------------------------------------------------------------------------------------- |
| Discord        | `{ "content": "Sanctifier scan completed ..." }`                                                    |
| Slack          | `{ "text": "Sanctifier scan completed ...", "attachments": [{ "color": "...", "fields": [...] }] }` |
| Teams          | `{ "text": "Sanctifier scan completed ..." }`                                                       |
| Custom webhook | Raw JSON payload: `event`, `project_path`, `timestamp_unix`, `summary`                              |

The shared payload data includes:

| Field                    | Description                                      |
| ------------------------ | ------------------------------------------------ |
| `event`                  | Always `scan.completed`                          |
| `project_path`           | Analysed path passed to `sanctifier analyze`     |
| `timestamp_unix`         | Completion timestamp as UNIX seconds             |
| `summary.total_findings` | Total number of findings emitted                 |
| `summary.has_critical`   | Whether any critical findings were detected      |
| `summary.has_high`       | Whether any high-severity findings were detected |

Webhook delivery failures are non-fatal: Sanctifier logs a warning to stderr and
still returns the analysis result.

---

### `sanctifier init`

Generate a `.sanctify.toml` configuration file in the current directory.

```bash
sanctifier init [OPTIONS]
```

| Flag      | Short | Default | Description                       |
| --------- | ----- | ------- | --------------------------------- |
| `--force` | `-f`  | `false` | Overwrite an existing config file |

---

### `sanctifier badge`

Create an SVG badge and optional Markdown snippet from a JSON scan report.

```bash
sanctifier badge [OPTIONS]
```

| Flag                       | Short | Default                   | Description                      |
| -------------------------- | ----- | ------------------------- | -------------------------------- |
| `--report <PATH>`          | `-r`  | `sanctifier-report.json`  | Path to a Sanctifier JSON report |
| `--svg-output <PATH>`      | —     | `sanctifier-security.svg` | Output SVG file                  |
| `--markdown-output <PATH>` | —     | —                         | Output Markdown snippet file     |
| `--badge-url <URL>`        | —     | local SVG path            | Public URL for the SVG           |

```bash
sanctifier analyze . --format json > sanctifier-report.json
sanctifier badge --report sanctifier-report.json \
  --svg-output badges/sanctifier-security.svg \
  --markdown-output badges/sanctifier-security.md
```

---

### `sanctifier callgraph`

Generate a Graphviz DOT call graph of cross-contract calls
(`env.invoke_contract`).

```bash
sanctifier callgraph [OPTIONS] [PATH]
```

| Flag              | Short | Default         | Description                                  |
| ----------------- | ----- | --------------- | -------------------------------------------- |
| `[PATH]`          | —     | `.`             | Contract directory, workspace, or `.rs` file |
| `--output <FILE>` | `-o`  | `callgraph.dot` | Output DOT file                              |

```bash
sanctifier callgraph ./contracts/amm-pool -o amm-callgraph.dot
dot -Tpng amm-callgraph.dot -o amm-callgraph.png   # requires Graphviz
```

---

### `sanctifier update`

Check for and download the latest Sanctifier binary from crates.io.

```bash
sanctifier update
```

---

### `sanctifier report`

Generate a security report (writes to stdout or a file).

```bash
sanctifier report [OPTIONS]
```

| Flag              | Short | Default | Description      |
| ----------------- | ----- | ------- | ---------------- |
| `--output <PATH>` | `-o`  | stdout  | Output file path |

---

## 📋 Example JSON Output

```bash
sanctifier analyze ./contracts/vulnerable-contract --format json
```

```jsonc
{
  "metadata": {
    "version": "0.1.0",
    "timestamp": "2026-03-24T12:00:00Z",
    "project_path": "./contracts/vulnerable-contract",
    "format": "sanctifier-ci-v1",
  },
  "summary": {
    "critical": 0,
    "high": 0,
    "medium": 2,
    "low": 0,
    "info": 0,
  },
  "findings": {
    "auth_gaps": [{ "code": "S001", "function": "src/lib.rs:set_admin" }],
    "panics": [
      {
        "code": "S002",
        "type": "expect",
        "location": "src/lib.rs:set_admin_secure",
      },
    ],
    "arithmetic_issues": [],
    "storage_collisions": [
      { "code": "S005", "value": "admin", "type": "storage::set (instance)" },
    ],
    "upgrade_admin_risks": [
      { "code": "S010", "category": "Governance", "function": "set_admin" },
    ],
  },
  "error_codes": [
    { "code": "S001", "category": "authentication", "description": "..." },
    "...",
  ],
  "vuln_db_matches": [],
  "schema_version": "1.0.0",
}
```

> **Note:** The example above is abbreviated. See the full field reference in the JSON Schema below.

---

## 📐 JSON Schema

The machine-readable contract for `--format json` output is published at
[`schemas/analysis-output.json`](schemas/analysis-output.json) (JSON Schema draft-07).

```
schemas/
└── analysis-output.json   # Draft-07 schema — validated in CI
```

Downstream tools (dashboards, IDE plugins, CI integrations) can use the schema to:

- **Validate** report files before processing them.
- **Generate** typed bindings in any language that supports JSON Schema.
- **Detect breaking changes** when the tool is upgraded.

The `schema_version` field in every report (e.g. `"1.0.0"`) is bumped independently
of the Sanctifier tool version whenever the output shape changes, so consumers can
guard on it without coupling to the CLI release cycle.

---

## 🤖 GitHub Action

Sanctifier ships a composite GitHub Action (`action.yml`) so CI consumers can
integrate with a few lines.

```yaml
name: Sanctifier Scan

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read
  security-events: write

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run Sanctifier
        uses: HyperSafeD/Sanctifier@main
        with:
          path: .
          format: sarif
          min-severity: high
          upload-sarif: "true"
          sarif-output: sanctifier-results.sarif
```

When `format: sarif` and `upload-sarif: "true"`, the action uploads the SARIF
file via `github/codeql-action/upload-sarif@v3` so findings appear in GitHub
code scanning.

---

## 📂 Project Structure

```text
Sanctifier/
├── contracts/              # Soroban smart contracts (examples & test targets)
├── frontend/               # Next.js web dashboard
├── schemas/
│   └── analysis-output.json  # JSON Schema (draft-07) for --format json output
├── tooling/
│   ├── sanctifier-cli/     # CLI binary (this is what you install)
│   └── sanctifier-core/    # Static-analysis engine & Z3 backend
├── data/
│   └── vulnerability-db.json  # Community-sourced vulnerability patterns
├── scripts/                # Deployment & CI helper scripts
└── docs/                   # Architecture decisions, guides, case studies
```

---

## ⚙️ Configuration

Run `sanctifier init` to generate a `.sanctify.toml`:

```toml
ignore_paths = ["target", ".git"]
enabled_rules = ["auth_gaps", "panics", "arithmetic", "ledger_size"]
ledger_limit = 64000
approaching_threshold = 0.8
strict_mode = false

[[custom_rules]]
name = "no_unsafe_block"
pattern = 'unsafe\s*\{'
severity = "error"

[[custom_rules]]
name = "no_mem_forget"
pattern = "std::mem::forget"
severity = "warning"
```

---

## 🏅 Add a Sanctifier Badge to Your Project

Display your contract's security status directly in your repository README.

### Step 1 — Generate the report and badge

```bash
# Produce a JSON report
sanctifier analyze . --format json > sanctifier-report.json

# Generate the SVG badge and a Markdown snippet
sanctifier badge \
  --report sanctifier-report.json \
  --svg-output .github/badges/sanctifier-security.svg \
  --markdown-output .github/badges/sanctifier-security.md \
  --badge-url "https://raw.githubusercontent.com/<owner>/<repo>/main/.github/badges/sanctifier-security.svg"
```

### Step 2 — Commit the badge to your repository

```bash
git add .github/badges/sanctifier-security.svg
git commit -m "ci: add Sanctifier security badge"
```

### Step 3 — Embed the badge in your README

Copy the snippet from `sanctifier-security.md`, or paste directly:

```markdown
[![Sanctifier: Secure](https://raw.githubusercontent.com/<owner>/<repo>/main/.github/badges/sanctifier-security.svg)](https://github.com/HyperSafeD/Sanctifier)
```

### What the badge reflects

| Badge color | Status       | Meaning                                             |
| ----------- | ------------ | --------------------------------------------------- |
| 🟢 Green    | **Secure**   | Zero findings                                       |
| 🟠 Orange   | **Warning**  | At least one finding (high severity or any finding) |
| 🔴 Red      | **Critical** | At least one critical-severity finding              |

The badge is regenerated automatically whenever you re-run `sanctifier badge`
with an updated report. Add it to your CI pipeline so the badge always
reflects the latest scan result.

---

## 📚 Documentation

| Document                                                           | Description                                 |
| ------------------------------------------------------------------ | ------------------------------------------- |
| [Getting Started](docs/getting-started.md)                         | First-run walkthrough                       |
| [Error Codes](docs/error-codes.md)                                 | Full finding-code reference                 |
| [Runtime Guards Integration](docs/runtime-guards-integration.md)   | Adding runtime guards to your contract      |
| [CI/CD Setup](docs/ci-cd-setup.md)                                 | GitHub Actions integration                  |
| [Soroban Deployment](docs/soroban-deployment.md)                   | Deploy guard contracts to testnet           |
| [Contributing Analysis Rules](docs/Contributing-analysis-rules.MD) | Writing custom analysis rules               |
| [Case Studies](docs/case-studies/soroban-examples.md)              | Benchmark against official Soroban examples |
| [Architecture Decisions](docs/adr/)                                | ADRs for design choices                     |

---

## 🤝 Contributing

We welcome contributions from the Stellar community! Please see the
[Contributing Guide](CONTRIBUTING.md) for details on setting up your
development environment, running tests, and submitting pull requests.

---

## 📄 License

MIT
