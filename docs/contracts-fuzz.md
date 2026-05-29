# Contracts Fuzz Harness CI

## Overview

The `.github/workflows/contracts-fuzz.yml` workflow runs the property-based
and coverage-guided fuzzers that target the `contracts/*` boundary code.  It
covers two harness families:

1. **Bolero in-process tests** — `cargo test`-shaped property tests in
   `contracts/my-contract/src/fuzz.rs`.  These run on every PR and push under
   stable Rust.
2. **cargo-fuzz / cargo-bolero coverage-guided harness** — `libFuzzer`
   targets in `contracts/my-contract/fuzz/fuzz_targets/`.  These run nightly
   on a scheduled cron under nightly Rust.

The shared invariant exercised by every harness is:

> The cross-contract message parser
> ([`my_contract::handle_cross_contract_message`](../contracts/my-contract/src/cross_contract.rs))
> must never panic on any byte sequence — it must either return a structured
> [`CrossContractMessage`] or a typed [`CrossContractError`].

This is the safe-by-default contract that all upstream callers (other Soroban
contracts and off-chain RPC) rely on.

## Jobs

### `fuzz-smoke` (every PR / push)

Runs on `ubuntu-latest` under stable Rust.  Two steps:

| Step | Command | Purpose |
|------|---------|---------|
| Bolero + e2e | `cargo test -p my-contract --tests` | Drives `src/fuzz.rs` and `tests/cross_contract_fuzz_e2e.rs` |
| Compile fuzz target | `cargo check --bin fuzz_cross_contract` (in `contracts/my-contract/fuzz`) | Guarantees the libFuzzer harness still builds |

The smoke job keeps PR feedback under a couple of minutes while ensuring no
regression breaks the deeper nightly harness before it gets a chance to run.

### `fuzz-deep` (scheduled, nightly 02:00 UTC)

Runs on `ubuntu-latest` under nightly Rust, time-boxed to 30 minutes.

| Step | Command | Time budget |
|------|---------|-------------|
| Bolero raw bytes | `cargo bolero test fuzz_raw_bytes_no_panic --time 60s` | 60s |
| Bolero structured | `cargo bolero test fuzz_structured_message_no_panic --time 60s` | 60s |
| cargo-fuzz target | `cargo fuzz run fuzz_cross_contract -- -max_total_time=120` | 120s |

If a target produces a corpus file or a crash artifact the run uploads the
`corpus/` and `artifacts/` directories as a `fuzz-artifacts` workflow
artifact (14-day retention) so the regression can be reproduced locally with:

```bash
cd contracts/my-contract
cargo bolero test <target> --corpus-dir <downloaded-corpus>
# or
cargo fuzz run fuzz_cross_contract <crash-input>
```

## Local development

Reproduce the smoke job:

```bash
cargo test -p my-contract --tests
( cd contracts/my-contract/fuzz && cargo check --bin fuzz_cross_contract )
```

Reproduce the deep job (requires nightly):

```bash
rustup toolchain install nightly
cargo +nightly install --locked cargo-bolero cargo-fuzz

cd contracts/my-contract
cargo +nightly bolero test fuzz_raw_bytes_no_panic --time 60s
cargo +nightly bolero test fuzz_structured_message_no_panic --time 60s
cargo +nightly fuzz run fuzz_cross_contract -- -max_total_time=120
```

## Wire format stability

The cross-contract message wire format is documented in
[`cross_contract.rs`](../contracts/my-contract/src/cross_contract.rs).  Any
change to opcode bytes, field ordering, or field widths is a **breaking
change** and requires:

- a `my-contract` minor-version bump (or major, if downstream contracts
  consume the format off-chain),
- migration notes in `CHANGELOG.md`,
- updates to all integration tests in
  `contracts/my-contract/tests/cross_contract_fuzz_e2e.rs`.
