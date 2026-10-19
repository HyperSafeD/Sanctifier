# ADR 011: Formal-Verification Scope for ZK Contracts

## Status

Accepted

## Context

The Sanctifier project is adding formal-verification capabilities across multiple
dimensions — Kani bounded model-checking (`#1211`, `#1214`), Z3 SMT proving
(`#1213`), and invariant-based verification (S011).  These tools prove meaningful
properties about contract-logic correctness: access control, state-transition
safety, arithmetic bounds, and invariant preservation over all reachable states.

However, there is a realistic risk that "formally verified" gets applied loosely
to ZK contracts once these tools land, when in truth the proofs cover
contract-logic correctness and **exclude** the deep cryptographic soundness of
Groth16, PLONK, or any other proving system.

This ADR explicitly scopes what "formally verified" means for ZK contracts in
this project so that the team, auditors, and users share a precise understanding.

## What is covered

### Contract-logic correctness (Kani)

Kani's bounded model-checking proves, for the specific harnesses written:

| Property | Example |
|----------|---------|
| No panic / overflow on valid inputs | `transfer_pure` never panics for valid balances |
| Authorization guards fire correctly | `initialize` fails after first call |
| State transitions preserve invariants | `total_supply == a + b` after every operation |
| Multi-step protocol correctness | VK rotation requires quorum AND timelock |

### Arithmetic safety (Z3 SMT)

Z3's SMT solver proves, for the modelled constraint set:

| Property | Example |
|----------|---------|
| No integer overflow in bounded arithmetic | `a * b / d` fits in u128 |
| Invariant violation reachability | `a + b` can overflow u64 |
| Circuit signal bound | Signal `x` is provably ≤ 2^64 - 1 under the accumulated R1CS constraints |

### Static analysis (Z-rules)

The Z-rule engine (Z001–Z014) detects the **presence** of security-relevant
patterns — nullifier checks, public-input binding, VK integrity checks, etc.
These checks are heuristic (pattern-based) and do not constitute formal proof
that the pattern is correctly implemented.

## What is NOT covered

### Cryptographic soundness of the proving system

Kani, Z3, and Sanctifier's static analysis do **not** prove:

1. **Groth16 knowledge-soundness** — that a prover cannot forge a valid proof
   without knowing a satisfying witness.  This is a property of the pairing-based
   argument system itself, not of any contract that calls a verifier.

2. **PLONK / Marlin / etc. soundness** — the same limitation applies to every
   non-trivial argument system.

3. **zk-SNARK security assumptions** — the proofs rely on:
   - The hardness of the discrete-log problem in elliptic-curve groups.
   - The security of the random-oracle model (Fiat-Shamir transform).
   - The assumption that the trusted setup ceremony was conducted honestly
     (no toxic-waste leakage).

4. **Implementation correctness of the verifier precompile** — if the Soroban
   host function that computes the ate pairing has a bug, all contract-level
   proofs are moot.

5. **WASM binary integrity** — the contract binary deployed on-ledger may differ
   from the source that was verified.  Reproducible builds and source-verification
   pipelines are out of scope of formal verification.

### What this means in practice

A contract that passes all Z-rules *and* has Kani/Z3 proofs for its business
logic is **safer than one without**, but it is **not provably secure** in the
cryptographic sense.  Specifically:

- The proof that "the nullifier check fires before the state transition" does
  not prove that the nullifier check correctly implements the zk-SNARK's
  nullifier derivation.
- The proof that "VK rotation requires multisig quorum" does not prove that the
  Groth16 verifier correctly rejects proofs under a replaced VK (that's a
  cryptographic property of the verification equation).
- The Z3 proof that "signal x ≤ 2^64" does not prove that the circuit's
  constraint system is sound (that requires full R1CS-to-SMT encoding, which
  is NP-hard for arbitrary circuits).

## Recommendations

1. **Audits remain mandatory** — formal verification complements, does not
   replace, a professional cryptographic audit.
2. **Claim precision** — marketing or documentation should say "formally verified
   contract-logic properties (access control, state transitions)" not "formally
   verified ZK contract".
3. **Scope documentation** — every Kani proof harness should document what it
   assumes and what it proves (see `contracts/kani-poc/` for examples).
4. **Bug bounty** — even with formal verification, a bug-bounty program is
   recommended for the proving-system integration layer.

## Cross-references

- ADR 006: Z3 Formal Verification — the SMT backend, its capabilities and
  limitations.
- `docs/kani-integration.md` — Kani integration strategy and the "Core Logic
  Separation" pattern.
- `docs/rules/Z001.md`–`Z014.md` — each Z-rule doc links here for scope
  clarification.
- `contracts/kani-poc/` — example of documented proof scope.
- `contracts/zk-verifier/` — reference Groth16 verifier with documented
  limitations.

## Consequences

**Positive:**
- Clear shared vocabulary for what "formally verified" means.
- Prevents over-trusting of verification results by auditors and users.
- Makes each proof harness's assumptions explicit.
- Provides a framework for external auditors (`#1112`) to evaluate the
  verification work.

**Negative:**
- The nuanced scope may be simplified or omitted in marketing copy (`#1170`),
  leading to the over-trust we aim to prevent.

## References

- [Kani Rust Verifier](https://model-checking.github.io/kani/)
- [Z3 Prover](https://github.com/Z3Prover/z3)
- [Groth16 (2016)](https://eprint.iacr.org/2016/260)
