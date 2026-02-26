# Formal Verification Video Tutorial Series

This page tracks a short video series for learning formal verification basics in Sanctifier with Kani.

## Series Overview

| Episode | Topic | Length | Video |
|---|---|---|---|
| 1 | Reading Sanctifier security reports | 8 min | `TBD` |
| 2 | Separating pure logic from Soroban host code | 10 min | `TBD` |
| 3 | Writing your first Kani proof harness | 12 min | `TBD` |
| 4 | Debugging failed proofs and fixing contracts | 12 min | `TBD` |

## Episode 1: Reading Sanctifier Security Reports

### Learning goals
- Understand report `summary` and `findings` sections
- Prioritize critical/high findings first
- Translate findings into concrete remediation tasks

### Demo flow
1. Run `sanctifier analyze ./contracts/kani-poc --format json`
2. Open the generated report and explain top-level metadata
3. Walk through auth gaps, panic issues, arithmetic issues, and storage warnings
4. Show how to track fixes issue-by-issue

## Episode 2: Separate Pure Logic from Soroban Host Code

### Learning goals
- Know why host-backed types (`Env`, `Address`, `Symbol`) are hard to verify directly
- Refactor contract logic into pure functions suitable for Kani

### Demo flow
1. Start from `contracts/kani-poc/src/lib.rs`
2. Isolate transfer/mint/burn checks into pure Rust functions
3. Keep `#[contractimpl]` methods thin and focused on host I/O

## Episode 3: Write Your First Kani Proof Harness

### Learning goals
- Add `#[kani::proof]` harnesses to verify invariants
- Use properties like conservation and insufficient-balance rejection

### Demo flow
1. Install Kani:
   - `cargo install --locked kani-verifier`
   - `cargo kani setup`
2. Add/inspect harnesses under `contracts/kani-poc`
3. Run `cargo kani --package kani-poc-contract`
4. Interpret pass/fail output

## Episode 4: Debug Failed Proofs and Patch the Contract

### Learning goals
- Read a failing counterexample
- Patch business logic to satisfy the invariant
- Re-run verification and confirm fix

### Demo flow
1. Introduce an intentional bug in pure logic
2. Run Kani and inspect failure trace
3. Apply fix and re-run proof
4. Re-scan with Sanctifier to ensure no regressions

## Recording Checklist

- Resolution: 1080p
- Include terminal font at readable size (>=16px equivalent)
- Keep each episode under 12 minutes
- Add chapter markers for setup, demo, and recap
- Update the `Video` column with final links after upload
