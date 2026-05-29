# Contracts

This directory contains the Soroban contracts used by Sanctifier for analysis, fixture generation, and security regression testing.

## Contract catalog

- `amm-pool`: Automated market-maker fixture for slippage and reserve-invariant checks.
- `bridge`: Cross-domain transfer fixture for authorization and replay analysis.
- `flashloan-token`: Flash-loan execution fixture for temporary-balance safety checks.
- `governance`: Proposal and voting fixture for control-plane validation.
- `kani-poc`: Model-checking fixture used by formal verification workflows.
- `multisig`: Multi-party authorization fixture for threshold and signer-order checks.
- `my-contract`: Baseline token-like contract used for deterministic testing/fuzzing examples.
- `oracle`: Price-feed fixture for stale-data and trust-boundary validation.
- `proxy`: Upgradeability fixture that emits governance-relevant upgrade/admin events.
- `reentrancy-guard`: Reentrancy defense fixture contract.
- `runtime-guard-wrapper`: Runtime guard and monitoring wrapper fixture.
- `shadowing-example`: Demonstrates variable-shadowing patterns and safer alternatives.
- `timelock`: Delayed execution fixture for governance sequencing.
- `token-with-bugs`: Intentionally vulnerable token fixture for negative-path validation.
- `unsafe-prng-example`: Fixture exposing predictable randomness usage.
- `vesting`: Vesting flow fixture for time-gated token release logic.
- `vulnerable-contract`: Intentionally unsafe contract used to verify detector coverage.

## Fixture notes

- Event-emission fixture notes live in `runtime-guard-wrapper` and `proxy`.
- Storage-collision fixture notes live in `runtime-guard-wrapper` and `shadowing-example`.
- Unhandled-`Result` fixture notes live in `runtime-guard-wrapper` and `token-with-bugs`.
- SEP-41 conformance fixtures live in `my-contract` and `fixtures/finding-codes/s012_token_interface.rs`.
## Structure
- `vulnerable-contract/`: A reference implementation demonstrating common security pitfalls Sanctifier can detect.
- `fixtures/finding-codes/`: Scan fixtures mapped to `S001` through `S012`.

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
