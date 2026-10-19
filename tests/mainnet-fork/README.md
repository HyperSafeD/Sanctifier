# Mainnet Fork Integration Tests

This directory contains the read-only mainnet-fork integration test suite for Sanctifier.

## Purpose

All other test fixtures are synthetic or hand-written. Real mainnet contracts often
contain patterns (macro usage, complex generics, large codebases, multi-hop call
chains) that curated fixtures don't exercise. This suite periodically fetches a
curated set of publicly deployed Soroban contracts from Stellar mainnet and runs the
full Sanctifier analysis engine against them — read-only, no state mutation, no
transactions submitted.

The goal is to catch crashes, hangs, and unexpected panics before they surface in
production user workflows.

## Files

| File | Description |
|------|-------------|
| `corpus.json` | Curated manifest of real mainnet Soroban contract addresses. |
| `README.md` | This file. |

The actual test code lives in:
```
tooling/sanctifier-core/tests/mainnet_fork_test.rs
```

The CI workflow lives in:
```
.github/workflows/mainnet-fork-ci.yml
```

## How It Works

1. **Corpus curation** — `corpus.json` lists publicly-deployed Soroban contracts with
   their contract IDs, expected-findings hints, and skip flags for network
   unavailability.

2. **WASM retrieval** — The CI job uses the Stellar Horizon REST API
   (`/accounts/{contract_id}`) to resolve the current WASM hash, then fetches
   the uploaded WASM binary from
   `https://horizon.stellar.org/soroban/wasm/{wasm_hash}`. The WASM is cached
   under `target/mainnet-fork-cache/` keyed by content hash.

3. **Static analysis** — The fetched WASM (or its embedded source if available via
   the contract's metadata) is decoded and fed to `sanctifier-core`'s
   `Analyzer::run_all`. Because Soroban contracts are compiled from Rust, the engine
   applies its full rule set.

4. **Crash/hang detection** — Each contract is analysed under a per-contract timeout
   (default 120 s). Any panic or timeout is captured and surfaced as a CI failure,
   with a structured log written to `target/mainnet-fork-report.json`.

5. **Expected vs actual findings** — The corpus manifest records optional
   `expected_findings` entries. If the engine returns findings outside the known set
   the job emits a warning (not a hard failure) for manual triage.

## Corpus Curation Policy

A contract is eligible for inclusion if:

- It is **publicly deployed** on Stellar mainnet with a known contract ID.
- It is **open-source** (source URL provided).
- It represents a **real production workload** (token, AMM, lending, DEX router,
  governance) rather than a test fixture.
- The maintainer has **not opted out** of third-party static analysis.

Contracts are updated via PR whenever significant new Soroban protocols launch.

## Running Locally

> The tests are skipped by default unless the `SANCTIFIER_MAINNET_FORK` environment
> variable is set to `1`. This prevents accidental network calls during local
> development.

```bash
# Requires network access to Stellar Horizon mainnet API.
SANCTIFIER_MAINNET_FORK=1 cargo test --test mainnet_fork_test -p sanctifier-core -- --nocapture
```

To analyse a single contract entry from the corpus:

```bash
SANCTIFIER_MAINNET_FORK=1 SANCTIFIER_FORK_CONTRACT=soroswap-router \
  cargo test --test mainnet_fork_test -p sanctifier-core -- --nocapture
```

## CI Schedule

The workflow (`mainnet-fork-ci.yml`) runs:

- **Weekly** on Sundays at 02:00 UTC (scheduled).
- **On-demand** via `workflow_dispatch`.
- **Never** on pull-request pushes (to avoid rate-limiting Horizon).

Results are published as a workflow artifact (`mainnet-fork-report`) and any
crash/hang opens a GitHub issue automatically.

## Triage Guide

When the CI job flags a contract:

| Signal | Action |
|--------|--------|
| **Crash / panic** | Open a bug report in the engine with the contract ID and stack trace. |
| **Hang (timeout)** | Profile with `RUST_LOG=debug` and check for infinite loops in visitor code. |
| **Unexpected finding** | Review the finding; if valid, add to `expected_findings` in `corpus.json`. |
| **Network error** | Contract is `skip_if_unreachable: true` — re-run manually when Horizon recovers. |
