# Soroban Examples Case Study

## Summary

On March 24, 2026, Sanctifier was run against the official
[`stellar/soroban-examples`](https://github.com/stellar/soroban-examples)
repository as a reproducible benchmark.

- Crates scanned: 43
- Crates with findings: 39
- Crates without findings: 4
- Total findings: 662
- Authorization gaps: 15
- Panic issues: 66
- Arithmetic issues: 64
- Unhandled results: 161
- SMT issues: 0

This baseline is useful, but it does not support a "0 false positives" claim
yet. Instead, it gives us a public benchmark for precision work.

## Zero-Finding Crates

The following crates completed with no findings:

- `hello_world`
- `logging`
- `multisig_1_of_n_account/contract`
- `workspace/contract_a_interface`

## Highest-Noise Crates

The largest result sets came from:

- `privacy-pools/libs/lean-imt` with 104 findings
- `privacy-pools/contract` with 95 findings
- `liquidity_pool` with 87 findings
- `privacy-pools/cli/coinutils` with 61 findings
- `token` with 37 findings

The dominant categories across the benchmark were unhandled-result findings,
panic findings, and arithmetic findings.

## Precision Follow-Up

The scan highlighted several likely precision problems that were turned into
follow-up issues in this repository:

- Auth-gap investigation: `#347`
- Unhandled-result investigation: `#348`
- Arithmetic and panic investigation: `#349`

## Responsible Disclosure Status

This automated pass did not establish any confirmed vulnerabilities that were
ready for responsible disclosure to the Stellar team. The findings above need
manual triage to separate true positives from false positives before any
external report is made.

## Reproducing the Benchmark

The repository now includes:

- `.github/workflows/soroban-examples.yml` for CI execution
- `scripts/analyze-soroban-examples.py` for local or CI reproduction

The workflow clones the official examples repository, builds the Sanctifier CLI,
runs the analysis across each crate containing `src/lib.rs`, and uploads a JSON
summary artifact.
