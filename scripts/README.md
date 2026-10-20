# `scripts/`

Helper scripts for building, testing, releasing and deploying Sanctifier, plus the
Python and Node utilities used by the GitHub Action and by CI. Run everything from the
**repository root** unless a script says otherwise; most use paths relative to it.

This page lists every file here: what it does, the runtime and tools it needs, and what
(if anything) invokes it. Scripts that live elsewhere but follow the same pattern are
listed at the end.

## Runtimes at a glance

| Kind | Files | What they need |
|------|-------|----------------|
| **Python** | `action_inputs.py`, `action_summary.py`, `resolve_action_version.py`, `analyze-soroban-examples.py` | Python **3.7+**. Standard library only. |
| | `compare-benchmarks.py` | Python **3.6+**. Standard library only. |
| **Node.js** | `validate_docs_specs.js`, `validate_release_artifacts.js` | Any maintained Node (run on 18, 20, 22, 24 and 26). CommonJS, built-in modules only. |
| | `validate_db.js` | Same Node range, **plus** `npm ci` at the repo root: it `require`s `ajv`, `ajv-formats` and `yaml`. `ajv` is not listed in `package.json` directly; it arrives through the `ajv-cli` dev dependency. |
| **Bash** | every `*.sh` | `bash`. External tools are listed per script below. |
| **PowerShell** | `generate-license-report.ps1` | PowerShell (Windows or `pwsh`). |

The Python minimums come from static analysis of the syntax and standard-library calls
each file uses (`vermin`), not from a pin in the repository.

### Which version CI actually uses

Nothing in the repository pins a single Node or Python version: there is no `.nvmrc`,
`.node-version`, `.tool-versions` or `engines` field. Each workflow chooses for itself:

| Runtime | Where it comes from |
|---------|---------------------|
| **Node 24** | `actions/setup-node` in the `ci` job (which runs `make lint`, and so `validate_db.js` and `validate_release_artifacts.js`), the docs/specs job (`validate_docs_specs.js`), `provenance.yml`, `lockfile-checks.yml` and `release.yml`. |
| **Node 20** | The license-compliance job in `ci.yml` only. |
| **Node, unpinned** | `mainnet-monitor.yml`, `testnet-monitor.yml` and `soroban-deploy.yml` run `.github/scripts/*.js` **without** `setup-node`, and so does the WASM job in `ci.yml`; they get whatever Node the `ubuntu-latest` runner image ships. `alert.js` uses the global `fetch`, so it needs Node 18 or newer. |
| **Python 3.x (latest)** | `actions/setup-python` with `python-version: "3.x"` in `test-action.yml`, `e2e-coverage.yml` and `code-scanning-sanctifier.yml`. |
| **Python, unpinned** | The `ci` job's `python -m unittest` step and `soroban-examples.yml` call `python` with no `setup-python`, so they use the runner image's Python. |
| **Python, user's runner** | The composite action (`action.yml`) runs `python` on the **consumer's** runner and sets nothing up itself. That runner needs Python 3.7+ available as `python`. |

If you add a script that needs a specific runtime, either pin it in the workflow that
runs it or record the requirement here.

## Python and Node utilities

| Script | What it does | Runtime / tools | Invoked by |
|--------|--------------|-----------------|------------|
| `action_inputs.py` | Validates and normalises the GitHub Action's inputs (`format` of `text`, `json` or `sarif`, severity names, safe relative `path`). | Python 3.7+ | `action.yml`; unit tests in `tests/action/` (`ci.yml`, `e2e-coverage.yml`); end to end via `test-action.yml` |
| `action_summary.py` | Runs `sanctifier analyze --format json`, counts SARIF findings and writes the action's step summary. | Python 3.7+; the `sanctifier` binary on `PATH` | `action.yml` |
| `resolve_action_version.py` | Turns the action ref and the requested version into a crates.io version string to install (rejects arbitrary `cargo install` arguments). | Python 3.7+ | `action.yml` |
| `analyze-soroban-examples.py` | Runs the CLI over every contract in a checkout of the official Soroban examples and writes a JSON summary. Usage: `<examples-root> <sanctifier-cli> <output-json>`. | Python 3.7+; a built `sanctifier` | `soroban-examples.yml` |
| `compare-benchmarks.py` | Compares current benchmark results against a baseline and fails on a regression above 10%. | Python 3.6+ | Not called by any workflow; see also `update-baseline.sh` |
| `validate_db.js` | Validates `data/vulnerability-db.json` and the SARIF / severity / security-review YAML files against their JSON Schemas. | Node + `npm ci` (see above) | `npm run lint:db`, part of `make lint`, which `ci.yml` runs |
| `validate_docs_specs.js` | Checks the docs/specs integration coverage: that the required docs exist with their key sections (`DOCUMENTATION_INDEX.md`, the docs-specs, troubleshooting, API-reference and action-support-matrix docs), and that `ci.yml`, `package.json` and `action.yml` still contain what those docs describe. | Node | `npm run docs:specs:check`; the docs/specs job in `ci.yml` |
| `validate_release_artifacts.js` | Checks the artifact set declared in `data/release-manifest.json` against what is on disk (and against `CHECKSUMS.txt`). Exits 0 on success, 1 on a validation failure, 2 on an environment error. | Node | `npm run lint:release-artifacts`; part of `make lint`; `provenance.yml` |

## Shell and PowerShell scripts

| Script | What it does | Tools | Invoked by |
|--------|--------------|-------|------------|
| `build-contracts.sh` | Deterministic, reproducible build of all contract WASM files (fixed `SOURCE_DATE_EPOCH` and `RUSTFLAGS`). | `cargo`, `rustc`, `git` | Manual; described in `docs/contracts-ci.md` and `SECURITY.md` |
| `gen-contract-docs.sh` | Generates ABI / interface docs for every contract (`--check` verifies they are up to date; `SKIP_RUSTDOC=1` skips rustdoc). | `cargo`; `jq` optional | `make contract-docs`, `make contract-docs-check`; the contract-docs job in `ci.yml` |
| `generate-provenance.sh` | Writes `CHECKSUMS.txt` covering every artifact in `data/release-manifest.json`, atomically. | `jq`; `sha256sum` or `shasum` | `provenance.yml` |
| `verify-artifacts.sh` | Verifies every release artifact in the manifest is present and parseable (`--check` for CI). | `jq` | `provenance.yml` |
| `release.sh <version>` | Bumps the version across the Cargo and npm manifests. | `bash`, `sed` | Manual; see `RELEASE_CHECKLIST.md` |
| `deploy.sh` | Deploys a contract WASM (`--network`, `--wasm`, `--dry-run`, `--confirm-mainnet`). Needs `SOROBAN_SECRET_KEY`. | `soroban`, `cargo`, `jq`; `sha256sum` for `--dry-run` | `soroban-deploy.yml` |
| `deploy-soroban-testnet.sh` | Builds the runtime-guard wrapper, deploys it to testnet and runs continuous health checks. | `cargo`, `soroban`, `jq`, `curl`, `sha256sum`, `openssl` | Manual; see `docs/soroban-deployment.md` and related docs |
| `validate-runtime-guards.sh` | Test harness for a deployed runtime-guard wrapper contract. | `soroban` | Manual; also exercised by `test-script-validation.sh` |
| `test-script-validation.sh` | Tests the input validation of `deploy-soroban-testnet.sh` and `validate-runtime-guards.sh` against stub tools. | `bash` | Manual (no workflow runs it) |
| `setup.sh` | Contributor setup: checks for Rust, `soroban` and Node/npm and offers to install what is missing. | `rustup`/`cargo`, `curl`, `npm` | Manual |
| `setup-test-network.sh` | Starts a standalone Soroban network in Docker (container `soroban-standalone` on port 8000) for E2E tests. | `docker`, `curl` | Manual |
| `check-benchmarks.sh` | Runs the SMT latency benchmark and the Criterion benchmarks for `sanctifier-core`. | `cargo` | Manual |
| `update-baseline.sh` | Copies `benchmarks/current.json` over `benchmarks/baseline.json`. | `jq` | Manual |
| `measure-core-coverage.sh` | Measures `sanctifier-core` line coverage and exits 1 below 90%. | `cargo-tarpaulin`, `jq`, `bc` | Manual; see `docs/RELEASE_CHECKLIST.md` |
| `run-extended-fuzz.sh [hours] [target]` | Runs a long fuzz campaign (24 hours and all targets by default). | `cargo`, `cargo-fuzz`, `rustc` | Manual; see `docs/RELEASE_CHECKLIST.md` |
| `analyze-network-divergence.sh` | Summarises `test-results/test-results-{testnet,futurenet,mainnet-fork}.json` and exits 1 if any network failed. | `jq` | Not called by any workflow |
| `test-lsp-server.sh`, `test-lsp-integration.sh` | Smoke tests for the language server. The integration script expects `./target/debug/sanctifier`. | `cargo`, a built `sanctifier`, `timeout` (GNU coreutils; not on stock macOS) | Manual |
| `test-packages.sh <version>` | Post-release QA of four packaging channels (Homebrew, npm, Scoop, winget) for a released version. | `gh`, `npm`, `sha256sum`/`shasum`, and the channel tools | Manual; see `docs/POST_RELEASE_PACKAGE_UPDATE.md` |
| `generate-license-report.ps1` | Combines the `cargo-deny` and npm license-checker output into `LICENSE_COMPLIANCE_REPORT.md`. | PowerShell | Not called by any workflow |

The tool lists come from reading each script; a script may also call ordinary POSIX
utilities such as `sed`, `awk` and `grep`.

## Scripts that live elsewhere

| Path | Runtime | Invoked by |
|------|---------|------------|
| `.github/scripts/check_manifest_ranges.py` | Python 3.9+ | `lockfile-checks.yml` (runner's `python3`) |
| `.github/scripts/alert.js` | Node 18+ (global `fetch`) | `mainnet-monitor.yml`, `testnet-monitor.yml`, `soroban-deploy.yml` |
| `.github/scripts/mainnet-monitor.js`, `testnet-monitor.js` | Node (built-in modules only); they shell out with `spawnSync` | `mainnet-monitor.yml`, `testnet-monitor.yml` |
| `tooling/sanctifier-wasm/scripts/verify-csp-compliance.js` | Node (the WASM job has no `setup-node`, so it uses the runner's Node) | the WASM job in `ci.yml` |
