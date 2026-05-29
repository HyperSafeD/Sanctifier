# Contracts CI Matrix

## Overview

The `.github/workflows/contracts-ci.yml` workflow compiles, lints, and tests
every contract in `contracts/*` independently.  It is triggered on any push or
pull request that modifies a file under `contracts/` or the root `Cargo.toml` /
`Cargo.lock`.

## Jobs

### `contract-check` (matrix)

Runs on `ubuntu-latest` for each contract listed in the matrix:

| Step | Command | Purpose |
|------|---------|---------|
| Check | `cargo check -p <contract>` | Fast type-check; catches import and type errors |
| Lint | `cargo clippy -p <contract> -- -D warnings` | Enforce Clippy lints as errors |
| Test | `cargo test -p <contract>` | Run unit and integration tests |

`fail-fast: false` is set so every contract is evaluated even when one fails —
this surfaces all regressions in a single CI run.

### `contract-wasm` (matrix)

Builds a subset of contracts to `wasm32-unknown-unknown --release`.  Only
contracts whose `[dependencies]` section does **not** include
`soroban-sdk = { features = ["testutils"] }` are listed here; the `testutils`
feature pulls in `std`-only code that cannot be compiled for WASM.

The step also prints the size of the resulting `.wasm` file.

## Adding a new contract

1. Add the crate to `[workspace.members]` in the root `Cargo.toml`.
2. Add the **crate package name** (as declared in the contract's `Cargo.toml`)
   to the `contract-check` matrix in `.github/workflows/contracts-ci.yml`.
3. If the contract's production dependencies do **not** include `testutils`,
   also add it to the `contract-wasm` matrix.
4. Add a WASM size budget entry to `contracts/benchmark/src/budgets.rs` and
   `scripts/build-contracts.sh`.

## Deterministic builds

`scripts/build-contracts.sh` produces reproducible WASM binaries by:

- Pinning `SOURCE_DATE_EPOCH` to the latest git commit timestamp.
- Fixing `RUSTFLAGS` to `-C opt-level=z -C lto=fat -C codegen-units=1`.
- Enforcing the minimum Rust version declared in `[workspace.package]`.

Run with `--check` in CI to additionally validate WASM binary sizes against the
budgets in `contracts/benchmark/src/budgets.rs`.

## Performance budgets

Size budgets are defined in two places (kept in sync manually):

- `contracts/benchmark/src/budgets.rs` — Rust constants, referenced in
  benchmark tests and CI commentary.
- `scripts/build-contracts.sh` — shell `declare -A` table used by the
  size-check logic.

CPU instruction ceilings are documented in `contracts/benchmark/src/budgets.rs`
and enforced implicitly: if an operation exceeds the Soroban host's default
budget the test panics.

## Threat model notes (issue #597)

The CI matrix hardens the `contracts/*` area in the following ways:

| Threat | Mitigation |
|--------|-----------|
| Broken contract silently merged | `contract-check` job fails the PR |
| Clippy regression | `-D warnings` promotes warnings to errors |
| WASM size bloat (hidden feature creep) | `contract-wasm` reports sizes; `--check` blocks merges |
| Non-deterministic WASM output | `build-contracts.sh` fixes `SOURCE_DATE_EPOCH` + `RUSTFLAGS` |
| CPU/memory regression (exceeds Soroban budget) | Benchmark tests in `contracts/benchmark` catch this |
| Untested code path merged | Matrix runs `cargo test` for every contract independently |
