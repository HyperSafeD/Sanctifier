# ADR 012: ZK Toolchain Integration Path

## Status

Accepted

## Context

Zero-Knowledge (ZK) vulnerabilities require parsers and a static analysis model to detect issues like missing nullifier checks and weak Fiat-Shamir implementations. There are several ZK toolchains in the ecosystem (e.g., circom, arkworks, halo2, Noir). We need to decide which toolchains Sanctifier will support as a first-class citizen in v1, and which will be deferred to the future roadmap. This choice directly impacts the source languages our parsers need to support and the verifier contract patterns we prioritize. (See spike #1190 for research details).

## Decision

We will prioritize **circom + snarkjs** and **arkworks (Rust)** for v1 support.

- **circom + snarkjs:** Has mature ecosystem tooling and is widely used across the industry for ZK rollups and application circuits.
- **arkworks:** Given our existing Rust infrastructure and Soroban's native Rust environment, supporting arkworks aligns well with our current tech stack and ecosystem.

Other toolchains, such as **Noir** and **halo2**, will be deferred to the future roadmap.

## Consequences

1. **Parser Scope (#1227, #1228, #1229):** The parsers must be designed to parse Circom source files and Rust (for arkworks). We will not build parser support for Noir at this time.
2. **Example Contracts (#1216):** The first verifier contract patterns and test suites built will target Circom/snarkjs Groth16/Plonk verifiers and arkworks-based verifiers.
3. **Roadmap:** We must explicitly communicate to users that Noir and halo2 are not supported in the initial ZK ruleset release, managing expectations.
