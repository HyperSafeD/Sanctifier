# `scripts/`

Helper scripts for building, testing, releasing and deploying Sanctifier, plus the
Python and Node utilities used by the GitHub Action and by CI. Run everything from the
**repository root** unless a script says otherwise; most use paths relative to it.

This page lists the Python and Node utilities here: what each does, the runtime it needs,
and what (if anything) invokes it.

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
