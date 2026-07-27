# ZK Feature Roadmap

**Milestone:** [ZK Integration](https://github.com/HyperSafeD/Sanctifier/milestones?q=ZK+Integration)
**Status of this document:** scope summary for the current ZK wave. Updated as the wave lands.

Sixty-two ZK issues are landing in a single wave. Reading all of them to find out what
Sanctifier can actually do for a zero-knowledge contract today is not reasonable, so
this page is the short version: what ships now, what is documented but not yet wired,
and what is deliberately out of scope for this wave.

This is the ZK-feature counterpart to the [mainnet readiness checklist](../RELEASE_CHECKLIST.md) (#1115).

---

## Contents

- [The one-paragraph version](#the-one-paragraph-version)
- [In scope for this wave](#in-scope-for-this-wave)
  - [Z-rule catalogue](#z-rule-catalogue)
  - [Toolchain support](#toolchain-support)
  - [Example contracts and fixtures](#example-contracts-and-fixtures)
  - [CLI and output](#cli-and-output)
  - [Dashboard](#dashboard)
  - [Documentation](#documentation)
- [Deferred to a future wave](#deferred-to-a-future-wave)
- [Known gaps in this wave](#known-gaps-in-this-wave)
- [How to read a Z-finding](#how-to-read-a-z-finding)

---

## The one-paragraph version

This wave gives Sanctifier a **contract-side** ZK analysis capability: it reads the Rust
contract that verifies proofs on-chain — and, to a lesser degree, the circom or Noir
circuit behind it — and flags the recurring ways ZK integrations get broken. Nullifiers
that are never recorded. Public inputs that don't commit to the transaction. Verifying
keys with no ceremony provenance, or loaded from storage and trusted without a check.
It does **not** prove your circuit correct, and it does not replace a ZK audit. Treat it
the way you treat the S-rules: a fast, cheap pass that catches the classes of bug that
keep recurring, run on every commit, well before an auditor sees the code.

---

## In scope for this wave

### Z-rule catalogue

The canonical catalogue is **Z001–Z014**, defined by [`docs/rules/Z001.md`–`Z014.md`](rules/)
and mirrored in `data/vulnerability-db.json` and `data/sarif/rule-metadata.yaml`.

All fourteen are **specified and documented**. Not all fourteen have a detector yet —
the table below is the honest status, because a documented rule with no implementation
finds nothing:

| Code | Rule | Severity | Detector | Fixture |
|------|------|----------|----------|---------|
| [Z001](rules/Z001.md) | Missing nullifier / double-spend check | Critical | ✅ `zk_double_spend_risk` | ✅ |
| [Z002](rules/Z002.md) | Insecure or predictable randomness as circuit input | High | ⏳ documented | ✅ |
| [Z003](rules/Z003.md) | Missing public-input binding (proof malleability) | Critical | ✅ `missing_public_input_binding` | ✅ |
| [Z004](rules/Z004.md) | Unverified trusted-setup parameters ("toxic waste") | Critical | ✅ `hardcoded_trusted_setup` | ✅ |
| [Z005](rules/Z005.md) | Missing verifying-key integrity check | High | ✅ `missing_vk_integrity_check` | ✅ |
| [Z006](rules/Z006.md) | Missing proof nonce / uniqueness enforcement | High | ⏳ documented | — |
| [Z007](rules/Z007.md) | Under-constrained circuit inputs | Critical | ◐ `arkworks_circuit_missing_range_check` (arkworks only) | — |
| [Z008](rules/Z008.md) | Curve / field mismatch | Critical | ⏳ documented | — |
| [Z009](rules/Z009.md) | Unbounded proof-verification loop | High | ⏳ documented | ✅ |
| [Z010](rules/Z010.md) | Verifying-key rotation without access control | Critical | ⏳ documented | ✅ |
| [Z011](rules/Z011.md) | Commitment reuse without domain separation | High | ⏳ documented | — |
| [Z012](rules/Z012.md) | ZK property leak via public-output over-exposure | Medium | ⏳ documented | — |
| [Z013](rules/Z013.md) | Insufficient batch-validation in ZK-rollup transitions | Critical | ⏳ documented | — |
| [Z014](rules/Z014.md) | Missing Merkle-root inclusion-proof verification | Critical | ⏳ documented | — |

✅ implemented and registered · ◐ partial · ⏳ specified, detector not yet landed

Three further ZK detectors ship without a canonical Z-code, covering patterns found
while building the above: `zk_missing_constraint` (a ZK entry point that never asserts
anything), `zk_verifier_skippable` (a verifier call reachable only through one branch of
an `if`/`else`), and `zk_verification_result_ignored` (a discarded verification result).
Assigning them stable codes is tracked for the next wave.

### Toolchain support

| Toolchain | Support in this wave |
|---|---|
| **Soroban contracts (Rust)** | Full — this is where every Z-rule detector runs |
| **circom** | Parser for templates, signals, constraint operators, and component instantiation; unconstrained-signal analysis |
| **Noir** | Parser for functions, parameter visibility (`pub`), and assert statements |
| **arkworks** (`ConstraintSynthesizer`) | Range-check analysis on `generate_constraints`, via the existing `syn` pipeline |

### Example contracts and fixtures

- **`contracts/private-transfer/`** — a shielded transfer contract (shield / private
  transfer / unshield) with commitments and a nullifier set. Its proof verification is a
  **placeholder**, not a real pairing check; it exists to exercise the analysis, not to
  be deployed.
- **`contracts/zk-verifier/`** — a reusable nullifier set with explicit state tracking.
- **`contracts/fixtures/finding-codes/z0NN_*.rs`** — per-rule fixtures, each annotated
  inline with which functions trigger (❌) and which are clean (✅). The Z003–Z005
  fixtures are asserted against directly by the snapshot suite, so a fixture and its
  rule cannot drift apart silently.

### CLI and output

Z-findings flow through the existing pipeline with no new commands: `sanctifier analyze`
reports them, `--format sarif` and `--format json` emit them, `sanctifier explain Z003`
prints the code's description and remediation, and the SARIF rule metadata in
`data/sarif/rule-metadata.yaml` carries their names, severities, and help URIs.

Because the ZK rules do not need Z3, they work in a Z3-free build:

```bash
cargo install sanctifier-cli --locked --no-default-features
sanctifier analyze contracts/private-transfer --format sarif
```

### Dashboard

A dedicated ZK view (`frontend/app/playground/zk/`) with a Z-findings panel and a
constraint-graph visualisation of the parsed circuit.

### Documentation

- [ZK Security Guide](zk-security-guide.md) — the vulnerability classes and the secure
  patterns that avoid them, with the Z-rules that check each one.
- [ZK Integration Guide](ZK-INTEGRATION-GUIDE.md) — wiring Sanctifier into a circom +
  snarkjs or Noir project and its CI.
- [`docs/rules/Z001.md`–`Z014.md`](rules/) — one page per rule: what it detects, why it
  matters, vulnerable and safe examples, and what the check cannot see.
- [ADR: ZK feature integration](adr/) — the design decisions behind the above.

---

## Deferred to a future wave

Explicitly **not** shipping now. No dates are attached to any of these — they are
sequenced by contributor interest and by what the first wave's users actually hit.

**Additional toolchains**
- Halo2, plonky2/plonky3, gnark, and Leo/Aleo circuits.
- Non-Soroban on-chain verifiers (Solidity/EVM Groth16 verifiers, Solana programs).
- Direct `.r1cs` / `.zkey` / `.ptau` artifact inspection. Today the analysis reads
  source, not compiled setup artifacts.

**Deeper circuit analysis**
- Full constraint-system soundness analysis — proving a circuit is not
  under-constrained, rather than pattern-matching the common ways it goes wrong.
- Formal verification for circuits. The Z3-backed pass (S011) covers contract
  invariants; extending it to constraint systems is a substantially larger problem and
  is not attempted here.
- Witness-generation analysis (non-determinism, side channels in the prover).

**Circuit visualisation depth**
- The constraint graph renders structure. Interactive constraint-level debugging,
  signal tracing, and diffing two versions of a circuit are future work.

**Ceremony verification**
- Z004 checks that a verifying key *cites* a ceremony transcript. Actually fetching that
  transcript and replaying the contribution chain is out of scope — it needs network
  access and a full MPC verifier, neither of which belongs in a static analyser.

**Proof-system-specific verifier correctness**
- Checking that a hand-written on-chain pairing check is a correct Groth16 verifier.
  This wave's rules check how a verifier is *used*, not whether its arithmetic is right.

---

## Known gaps in this wave

Stated plainly so nobody discovers them the hard way:

1. **Nine of the fourteen Z-rules are documentation-only today.** See the table above.
   A clean `sanctifier analyze` run does not mean those nine classes are absent from
   your contract.
2. **Every Z-rule is a heuristic.** They pattern-match names, call shapes, and local
   dataflow. They will miss code that achieves the same thing by a different route, and
   they can fire on code that is safe for a reason the analyser cannot see. Read the
   "what it will not catch" section on each rule page.
3. **Circuit-side coverage is much thinner than contract-side coverage.** The circom and
   Noir parsers exist and are used, but most detectors run on the Rust contract.
4. **The example contracts do not implement real proof verification.** They are analysis
   fixtures. Do not copy their `verify_proof` into anything that holds value.
5. **Rule-code drift is still being cleaned up.** The canonical numbering is
   `docs/rules/` + `data/vulnerability-db.json` + `data/sarif/rule-metadata.yaml` +
   `finding_codes.rs`, all now aligned. If you find a stale Z-number anywhere else in
   the tree, it is a bug — please open an issue.

---

## How to read a Z-finding

A Z-finding says "this is a shape that has caused real losses". It does not say "your
contract is exploitable". The sequence that works:

1. Open the rule page (`docs/rules/Z0NN.md`) and read **why it matters** — the finding
   text is a summary, the page is the argument.
2. Check the **what it will not catch** section. It tells you what you still have to
   verify yourself, and often that is the more important half.
3. Compare against the fixture in `contracts/fixtures/finding-codes/`, which shows the
   triggering and clean forms side by side.
4. If it is a false positive, say so in an issue with the snippet. Every Z-rule
   heuristic in this wave was tightened by exactly that feedback.

---

**Related:** [ZK Security Guide](zk-security-guide.md) ·
[ZK Integration Guide](ZK-INTEGRATION-GUIDE.md) ·
[Rule catalogue](rules/) ·
[Wave dependencies](WAVE_DEPENDENCIES.md)
