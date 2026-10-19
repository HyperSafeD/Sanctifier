# Contracts

This directory contains the Soroban contracts used by Sanctifier for analysis, fixture generation, and security regression testing.

## Contract catalog

- `allowance-race`: Demonstrates the classic ERC-20 `approve → transferFrom` race
  condition with safe `increase_allowance` / `decrease_allowance` helpers.
- `amm-pool`: Automated market-maker fixture for slippage and reserve-invariant checks.
- `benchmark`: Performance benchmark harness with compile-time WASM-size and
  CPU-instruction ceilings for token and vesting contracts.
- `bridge`: Cross-domain transfer fixture for authorization and replay analysis.
- `deposit-withdraw`: Demonstrates the auth-gap vulnerability (S001) — a `withdraw`
  function without `require_auth` that can be drained by any caller.
- `flashloan-token`: Flash-loan execution fixture for temporary-balance safety checks.
- `governance`: Proposal and voting fixture for control-plane validation.
- `kani-poc`: Model-checking fixture used by formal verification workflows.
- `multisig`: Multi-party authorization fixture for threshold and signer-order checks.
- `multisig-no-recovery`: Demonstrates the stuck-state vulnerability — an N-of-M multisig
  with no signer rotation, guardian, or recovery path.
- `my-contract`: Baseline token-like contract used for deterministic testing/fuzzing examples.
- `oracle`: Price-feed fixture for stale-data and trust-boundary validation.
- `private-transfer`: Zcash-style shielded-balance private transfer demo using Groth16
  ZK proofs with hidden amounts and identities.
- `proxy`: Upgradeability fixture that emits governance-relevant upgrade/admin events.
- `reentrancy-guard`: Reentrancy defense fixture contract.
- `runtime-guard-wrapper`: Runtime guard and monitoring wrapper fixture.
- `security-disclaimers`: Standardized security disclaimers and safe usage guidelines
  module with runtime safety checks and security levels (CRITICAL/HIGH/MEDIUM/LOW).
- `sep41-compliance`: Standardized SEP-41 token-interface compliance suite that
  verifies all ten SEP-41 functions across contracts.
- `shadowing-example`: Demonstrates variable-shadowing patterns and safer alternatives.
- `test-support`: Shared test helpers (Env, token, asserts, SEP-41 compliance harness)
  to reduce CI compile times across contracts.
- `allowance-race`: ERC-20 style `approve` → `transfer_from` race fixture, with safe `increase_allowance`/`decrease_allowance` helpers.
- `amm-pool`: Automated market-maker fixture for slippage and reserve-invariant checks.
- `benchmark`: Benchmark harness that checks the SEP-41 token and vesting contracts stay within WASM-size and CPU-instruction budgets.
- `bridge`: Cross-domain transfer fixture for authorization and replay analysis.
- `deposit-withdraw`: Auth-gap fixture (S001) with an unauthenticated `withdraw_unsafe` and the one-line fix.
- `flashloan-token`: Flash-loan execution fixture for temporary-balance safety checks.
- `governance`: Proposal and voting fixture for control-plane validation.
- `kani-poc`: Model-checking fixture used by formal verification workflows.
- `multisig`: Multi-party authorization fixture for threshold and signer-order checks. See [`docs/multisig.md`](../docs/multisig.md) for the proposal flow, recovery path and what to look for in a Sanctifier report.
- `multisig-no-recovery`: Intentionally vulnerable multisig with no signer-recovery path, to contrast with `multisig`.
- `my-contract`: Baseline token-like contract used for deterministic testing/fuzzing examples.
- `oracle`: Price-feed fixture for stale-data and trust-boundary validation.
- `private-transfer`: Shielded-balance private transfer example (shield, transfer, unshield) that calls a Groth16 verifier.
- `proxy`: Upgradeability fixture that emits governance-relevant upgrade/admin events. See [`docs/proxy.md`](../docs/proxy.md) for the pattern, what Sanctifier reports on it, and how to reproduce.
- `reentrancy-guard`: Reentrancy defense fixture contract.
- `runtime-guard-wrapper`: Runtime guard and monitoring wrapper fixture.
- `security-disclaimers`: Shared security-disclaimer and safe-usage module used by the other contracts, with runtime configuration checks.
- `sep41-compliance`: Standard SEP-41 token-interface conformance suite run against every token contract.
- `shadowing-example`: Demonstrates variable-shadowing patterns and safer alternatives.
- `test-support` (package `sanctifier-test-support`): Shared Soroban test helpers (environment setup, mocked auth, token clients, SEP-41 harness) compiled once for all contract tests.
- `timelock`: Delayed execution fixture for governance sequencing.
- `token-with-bugs`: Intentionally vulnerable token fixture for negative-path validation.
- `unsafe-prng-example`: Fixture exposing predictable randomness usage.
- `vesting`: Vesting flow fixture for time-gated token release logic.
- `vulnerable-contract`: Intentionally unsafe contract used to verify detector coverage.
- `zk-verifier`: Reference Groth16 proof-verifier contract with nullifier-set storage,
  public-input binding, access-controlled VK rotation (multisig+timelock), and Kani
  proof harnesses (Z001, Z003, Z005, Z010).

## Fixture notes

- Event-emission fixture notes live in `runtime-guard-wrapper` and `proxy`. Upgrade-pattern notes for `proxy` are in [`docs/proxy.md`](../docs/proxy.md).
- Storage-collision fixture notes live in `runtime-guard-wrapper` and `shadowing-example`.
- Unhandled-`Result` fixture notes live in `runtime-guard-wrapper` and `token-with-bugs`.
- SEP-41 conformance fixtures live in `my-contract` and `fixtures/finding-codes/s012_token_interface.rs`.
## Structure
- `vulnerable-contract/`: A reference implementation demonstrating common security pitfalls Sanctifier can detect.
- `fixtures/finding-codes/`: Scan fixtures mapped to `S001` through `S012`.
- `scripts/`: Testnet deployment script (`testnet-deploy.sh`) for deploying contracts to testnet.
- `fixtures/`: Additional fixture contracts and XDR construction examples (`auth_gap_contract.rs`, `xdr_raw_construction.rs`, `versioning/`).
- `MIGRATION_NOTES.md`: Migration notes for security disclaimer updates across contracts.

## Other directories and files

These are not contract crates:

- `fixtures/`: Source-only scan fixtures. `fixtures/finding-codes/` maps to `S001` through `S012`; `fixtures/versioning/` documents the versioning conventions and named fixtures.
- `scripts/`: Helper scripts (`testnet-deploy.sh`).
- `MIGRATION_NOTES.md`: Notes on the security-disclaimers migration.

## Keeping this list current

Every directory under `contracts/` that has a `Cargo.toml` should appear in the catalog above. To check:

```bash
for d in contracts/*/; do [ -f "$d/Cargo.toml" ] && basename "$d"; done
```

## 🔐 Security Disclaimers

All contracts in this directory include standardized security disclaimers and safe usage guidelines:

- **Security Levels:** Critical, High, Medium, Low based on risk exposure
- **Disclaimer Categories:** Audit, Usage, Upgrade, Emergency
- **Runtime Validation:** Security configuration checks in sensitive operations
- **Documentation:** Comprehensive security considerations in each contract

For detailed implementation guidance, see [Contract Security Disclaimers Guide](../docs/contract-security-disclaimers.md).

## Development

Run tests for one contract:

```bash
cargo test -p runtime-guard-wrapper
```

Run analysis for the full contracts tree:

```bash
sanctifier analyze contracts
```

For finding-code focused fixture scans:
```bash
sanctifier analyze contracts/fixtures/finding-codes --format json
```

Run security disclaimer tests:

```bash
cargo test -p security-disclaimers
```

## Real-mainnet Fork Corpus

The contracts above are **synthetic fixtures** — hand-written or generated
specifically to exercise individual detectors.  Real mainnet contracts often
contain patterns (complex generics, large codebases, macro-heavy code,
multi-hop call chains) that synthetic fixtures don't exercise.

A complementary **read-only mainnet fork corpus** lives in
[`tests/mainnet-fork/`](../tests/mainnet-fork/README.md).  It curates real,
publicly-deployed Soroban contracts and runs the full analysis engine against
them periodically in CI — without submitting any transaction to mainnet.

See [`tests/mainnet-fork/corpus.json`](../tests/mainnet-fork/corpus.json) for
the list of contracts and
[`.github/workflows/mainnet-fork-ci.yml`](../.github/workflows/mainnet-fork-ci.yml)
for the scheduled CI job.
