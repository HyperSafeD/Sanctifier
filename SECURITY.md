# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities in Sanctifier seriously. If you discover a security issue, please follow the responsible disclosure process outlined below.

### How to Report

1. **Do not** open a public GitHub issue for security vulnerabilities.
2. Send an email to **security@sanctifier.dev** with the following information:
   - A clear description of the vulnerability.
   - Steps to reproduce the issue.
   - The affected component (CLI, core library, frontend, smart contracts).
   - The potential impact and severity assessment.
   - Any suggested fix or mitigation (optional).

### What to Expect

- **Acknowledgement**: You will receive an acknowledgement within **48 hours** of your report.
- **Assessment**: We will assess the vulnerability and determine its severity within **5 business days**.
- **Resolution**: We aim to release a fix within **30 days** of confirming the vulnerability, depending on complexity.
- **Disclosure**: We will coordinate with you on public disclosure timing. We request a **90-day disclosure window** from the initial report.

### Severity Levels

| Level    | Description                                              | Response Time |
|----------|----------------------------------------------------------|---------------|
| Critical | Remote code execution, data loss, authentication bypass  | 24 hours      |
| High     | Significant impact on analysis accuracy or data integrity| 3 days        |
| Medium   | Limited impact, requires specific conditions             | 7 days        |
| Low      | Minimal impact, informational                            | 30 days       |

### Scope

The following components are in scope for vulnerability reports:

- **sanctifier-core**: Static analysis engine
- **sanctifier-cli**: Command-line interface
- **Frontend dashboard**: Web-based visualization
- **Smart contract examples**: Only if vulnerabilities affect the analysis tooling itself

### Out of Scope

- Vulnerabilities in third-party dependencies (report these to the respective maintainers)
- Issues in the example vulnerable contracts (these are intentionally insecure for demonstration)
- Denial of service through excessively large input files

## Safe Harbor

We consider security research conducted in accordance with this policy to be:

- Authorized and welcome
- Conducted in good faith
- Not subject to legal action from our side

We will not pursue legal action against researchers who follow this responsible disclosure process.

## Recognition

We appreciate the security research community's efforts. With your permission, we will acknowledge your contribution in our release notes and security advisories.

---

## Contract Threat Model (CI Hardening — Issue #597)

This section documents the threat model for `contracts/*` and the mitigations
enforced by the CI compile matrix introduced in
[`.github/workflows/contracts-ci.yml`](.github/workflows/contracts-ci.yml).

### Assets and trust boundaries

| Asset | Threat | Trust boundary |
|-------|--------|---------------|
| User funds locked in vesting / multisig | Unauthorised withdrawal | Contract auth guards + Soroban host |
| Governance proposals | Vote manipulation | Quorum / threshold checks |
| Token balances | Integer overflow | `checked_add` / `checked_sub` everywhere |
| WASM bytecode | Supply-chain tampering | Deterministic builds; hash pinned in deployment manifest |
| CI pipeline | Malicious dependency | `Cargo.lock` committed; `cargo audit` in CI |

### Mitigations shipped with this change

| Threat | Mitigation |
|--------|-----------|
| Broken contract silently merged | Per-contract `cargo check` + `cargo test` in matrix CI |
| Clippy regression | `-D warnings` in `cargo clippy` step |
| WASM binary size bloat | `scripts/build-contracts.sh --check` enforces per-contract byte budgets |
| Non-deterministic WASM output | `SOURCE_DATE_EPOCH` + fixed `RUSTFLAGS` in build script |
| CPU/memory regression | Benchmark tests in `contracts/benchmark` exercise operations under the Soroban default resource budget |
| Untested code path merged | CI matrix runs tests for every contract independently |

### Contract-specific notes

- **vulnerable-contract** and **token-with-bugs** are intentionally insecure
  for analysis tooling demonstration.  They are **never** deployed; their
  presence in CI is to validate that the Sanctifier tooling correctly flags
  their weaknesses.
- **kani-poc** contains Kani proof harnesses.  These are excluded from
  standard `cargo test` but can be verified with `cargo kani` on a host with
  Kani installed.
- Contracts with `soroban-sdk = { features = ["testutils"] }` in
  `[dependencies]` (not `[dev-dependencies]`) cannot be compiled to
  `wasm32-unknown-unknown`; they are excluded from the WASM build job.  See
  [`docs/contracts-ci.md`](docs/contracts-ci.md) for the full list.
