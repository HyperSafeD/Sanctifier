# ZK-Verifier Contract Threat Model (Soroban)

**Status:** Proposed
**Issue:** #1195
**Feeds:** Z-rule set #1197–#1210 (see [`docs/zk-security-guide.md`](zk-security-guide.md), [`docs/rules/`](rules/))
**Related:** [ADR 010 — ZK Feature Integration](adr/010-zk-feature-integration.md), [ZK Roadmap](zk-roadmap.md), [ZK Private Transfer Case Study](case-studies/zk-private-transfer.md)

---

## Purpose

`vscode-extension/THREAT_MODEL.md` and `docs/github-action-threat-model.md` model the
attack surface of Sanctifier's own tooling. This document models a different surface:
the **contracts Sanctifier analyzes** — specifically, Soroban smart contracts that act
as on-chain ZK-proof verifiers (Groth16-style pairing checks, nullifier sets, Merkle
membership, rollup state transitions).

ZK-verifier contracts introduce a threat class that no `S0xx` rule and no prior threat
model in this repo covers: cryptographic trust assumptions (trusted setup, curve
parameters) baked into contract state, and proof-system semantics (malleability,
replay, soundness) that look like ordinary Rust to a source-level analyzer but carry
consequences ordinary Rust never does. Every threat below is written to answer one
question: **if this goes wrong on mainnet, what breaks, and which Sanctifier check
would have caught it?**

This document is the derivation record for the Z-rule set. Each threat maps to at
least one existing Z-rule (`Z001`–`Z014`, shipped in #1197, #1203, #1205, #1207, #1204,
#1208–#1210) or to a rule proposed here to close a gap the roadmap already flags as
open (see [§6](#6-gaps-and-newly-proposed-mitigations)).

---

## 1. Scope

**In scope:**

- Soroban contracts that verify zero-knowledge proofs on-chain (Groth16/PlonK-style
  pairing checks, nullifier/commitment set management, Merkle inclusion verification,
  ZK-rollup batch state transitions).
- The circom/Noir/arkworks circuit source that defines the statement being proved,
  to the extent Sanctifier's front-ends parse it (see [ADR 010](adr/010-zk-feature-integration.md)).
- Verifying-key and trusted-setup-parameter handling as contract state.

**Out of scope (see [§7](#7-non-goals)):**

- Whether the underlying pairing/arithmetic implementation is cryptographically
  correct (Sanctifier checks *usage*, not proof-system math).
- Off-chain prover security (side channels, witness leakage, MPC ceremony execution).
- Sanctifier's own tooling attack surface — that is covered by
  `vscode-extension/THREAT_MODEL.md` and `docs/github-action-threat-model.md`.

---

## 2. Assets

| Asset | Description | Where it lives |
|---|---|---|
| Verifying key (VK) | The cryptographic root of trust for every proof the contract accepts | Contract persistent storage or hardcoded bytes |
| Trusted-setup parameters | CRS / SRS / proving-key material the VK derives from | Off-chain ceremony transcript; VK is its on-chain fingerprint |
| Nullifier set | Records which proofs/notes have been consumed | Contract persistent storage |
| Public inputs | Transaction-context values (caller, recipient, amount, contract ID) bound into the proof | Constructed at call time, passed to the verifier |
| Merkle root(s) | Committed state used to prove set membership (UTXOs, allowlists, rollup state) | Contract persistent storage |
| Circuit constraints | The statement a proof attests to | circom/Noir/arkworks source, off-chain |
| Shielded value | Tokens or state locked behind a proof-gated release | Contract balance / storage |

## 3. Trust Boundaries

- **Prover (untrusted) → contract (trust boundary):** anyone can submit a `(proof,
  public_inputs)` pair to a public entry point. The contract must treat both as
  attacker-controlled.
- **Ceremony (trusted, but only if verifiable) → verifying key (trust boundary):** the
  VK is only as trustworthy as the ceremony that produced it. The contract has no way
  to verify this itself — it can only check that the VK it holds matches a
  provenance-documented value (Z004) and hasn't been swapped since (Z005).
- **Admin/governance (privileged) → verifying-key storage (trust boundary):** VK
  rotation is a privileged write. Anyone who can call it without `require_auth()`
  crosses from "prover" trust to "root of trust" trust (Z010).
- **Off-chain circuit source ↔ on-chain verifier (trust boundary):** the circuit
  defines the statement; the verifier enforces it. These are compiled and deployed
  separately and can drift — curve mismatch (Z008), under-constrained signals (Z007) —
  with no on-chain signal that they've diverged.

---

## 4. Threat Analysis

Each threat: what goes wrong, the mainnet blast radius if it does, a real-world
precedent grounding the risk, and the mitigation (existing Z-rule and/or contract
pattern).

### T1 — Trusted-Setup / Toxic-Waste Compromise

**Threat:** The verifying key derives from a trusted-setup ceremony. If the ceremony's
"toxic waste" (the random parameters that must be destroyed after generating the CRS)
was never destroyed, or the VK in contract storage was never checked against a
verifiable ceremony transcript, an attacker holding that waste can forge proofs for
**any false statement** — mint without collateral, spend without a valid nullifier,
pass any circuit constraint at will.

**Mainnet impact:** Critical, unbounded. A forged proof is indistinguishable from a
real one to the verifier; the contract has no second signal. Total loss of all value
gated behind the compromised VK.

**Real-world precedent:** Zcash's Sprout parameters used a large multi-party ceremony
specifically because a single dishonest participant retaining toxic waste can
counterfeit funds undetectably; Zcash also disclosed and patched a soundness bug in
the underlying BCTV14a construction (2018–2019, "Zcash Counterfeiting Vulnerability")
before it was exploited — both cases where a defect in the proof system's trust
assumptions would have allowed undetectable counterfeiting on a live network with real
value at stake.

**Mitigation:** [Z004](rules/Z004.md) `hardcoded_trusted_setup` — flags VK/CRS bytes
hardcoded with no adjacent, auditable ceremony-transcript reference. Contract pattern:
store the VK with a documented ceremony citation, never regenerate it on-chain.
*(Sanctifier checks provenance is cited, not that the ceremony transcript is valid —
replaying an MPC ceremony is explicitly out of scope; see [§7](#7-non-goals).)*

---

### T2 — Proof Malleability via Missing Public-Input Binding

**Threat:** A verifier that accepts `(proof, public_inputs)` without checking that
`public_inputs` corresponds to *this* transaction lets an attacker take someone else's
valid proof, substitute a different recipient/amount in the public inputs, and replay
it to redirect the proof's effect. Groth16-style proofs are also algebraically
malleable in some implementations (a valid proof can be re-randomized into a different
but still-valid proof for the same statement), compounding the risk if the contract
uses proof identity rather than input binding as its uniqueness anchor.

**Mainnet impact:** Critical. An attacker who observes one valid proof in the mempool
or on-chain can redirect its effect (e.g., a shielded transfer's recipient) without
knowing any secret witness.

**Real-world precedent:** The "Frozen Heart" vulnerability class (Trail of Bits /
Zellic, 2022) found weak Fiat-Shamir binding across multiple production
non-interactive proof systems (PlonK, Bulletproofs, Girault) — the general pattern of
"the proof doesn't cryptographically commit to everything the verifier assumes it
does" repeated across independent, audited implementations.

**Mitigation:** [Z003](rules/Z003.md) `missing_public_input_binding` — requires public
inputs to be constructed from caller, recipient, amount, and contract address at call
time (see the `build_public_inputs_*` pattern in
[`contracts/private-transfer`](../contracts/private-transfer/), documented in the
[case study](case-studies/zk-private-transfer.md)).

---

### T3 — Nullifier / Proof Replay

**Threat:** A valid proof only proves knowledge of a witness — it says nothing about
whether that witness has been "spent" before. Without a nullifier recorded and checked
*before* the state-changing effect, the same proof (or the same underlying note) can
be replayed to double-spend or double-claim.

**Mainnet impact:** Critical. Direct fund duplication; severity scales with the value
locked behind the entry point. The [private-transfer case study](case-studies/zk-private-transfer.md)
walks through this exact failure end-to-end.

**Real-world precedent:** Nullifier-based double-spend prevention is the core design
requirement in every shielded-pool protocol (Zcash, Tornado Cash, Semaphore); the
0xPARC ZK Bug Tracker catalogs multiple production audits where an application-layer
nullifier check was missing, malformed, or checked after (rather than before) the
state-changing effect.

**Mitigation:** [Z001](rules/Z001.md) `zk_double_spend_risk` — check-before-write
nullifier pattern (check spent flag → verify proof → effect → record nullifier).
[Z006](rules/Z006.md) extends this to proof-gated actions without an explicit
"balance" (votes, attestations) that still need nonce/epoch binding to prevent
cross-context replay.

---

### T4 — Verifying-Key Tampering (Integrity)

**Threat:** The VK is loaded from storage on every verification call. Storage
corruption, an upgrade bug, or (see T5) an unauthorized rotation can silently swap it
for an attacker-controlled key. If the contract never re-checks the loaded VK's
fingerprint, it will happily accept proofs generated under the attacker's fake key —
functionally identical to T1 but reachable without ever compromising a real ceremony.

**Mainnet impact:** Critical. Same blast radius as T1 (arbitrary forged proofs
accepted) but with a much lower bar to trigger — a storage bug or a single
unauthorized write, not an MPC compromise.

**Real-world precedent:** Same class of failure as any "swap the root of trust in
storage" bug family seen across bridge and rollup incidents (see T8's Nomad
precedent) — the specific mechanism differs, but the pattern of "a value the contract
implicitly trusts was writable without a subsequent integrity check" recurs across
chains.

**Mitigation:** [Z005](rules/Z005.md) `missing_vk_integrity_check` — hash-check the
loaded VK against a reference fingerprint committed at deployment, on every
verification call (see the `VkHash` pattern in
[`zk-security-guide.md §2.4`](zk-security-guide.md#24-trusted-setup-key-management)).

---

### T5 — Verifying-Key Rotation Without Access Control

**Threat:** If the function that writes a new VK to storage has no `require_auth()`
guard, *any* caller — not just governance — can replace the root of trust outright.
This is the write-side precondition for T4, and strictly more dangerous: no bug is
required, just a public call.

**Mainnet impact:** Critical. Equivalent to an unprotected `set_admin` (S001), but
worse: there is no on-chain event distinguishing a legitimate rotation from an attack
until forged proofs start landing.

**Real-world precedent:** Unprotected privileged-write functions are among the most
common findings in Soroban/EVM audits generally (mirrored by `S001` in this repo's own
`S`-rule set); applying the same scrutiny to VK rotation specifically is new because
audits historically treat "verifier configuration" as static and under-scrutinize it
relative to `set_admin`.

**Mitigation:** [Z010](rules/Z010.md) `unprotected_vk_rotation` — require
`admin.require_auth()` (or multisig-guardian auth) before any VK write.

---

### T6 — Under-Constrained Circuit Inputs

**Threat:** A signal used in arithmetic or comparison without an accompanying
range-check constraint can overflow the proof system's finite field. Because field
arithmetic wraps, a value the application logic assumes is bounded (e.g., a token
amount) can be crafted to satisfy `isValid <== amount < balance` while being
negative-equivalent mod the field prime — bypassing the check entirely while still
producing a valid proof.

**Mainnet impact:** Critical. The circuit "verifies" a statement the application never
intended to allow; downstream contract logic then acts on a false premise (e.g.,
approves a withdrawal the real balance can't cover).

**Real-world precedent:** Under-constrained-circuit bugs are the single largest
category in the 0xPARC ZK Bug Tracker (already cited as a core reference in
[`zk-security-guide.md §5`](zk-security-guide.md#5-further-reading)) — missing range
checks and missing equality constraints recur across independently-audited circom and
Halo2 circuits in production.

**Mitigation:** [Z007](rules/Z007.md) `arkworks_circuit_missing_range_check` (partial:
arkworks today, circom/Noir range-check detection tracked as a known gap in the
[roadmap](zk-roadmap.md#known-gaps-in-this-wave)) — flags signals used in comparisons
without a preceding range constraint.

---

### T7 — Curve / Field Confusion

**Threat:** The on-chain Rust verifier and the off-chain circuit must agree on the
same elliptic curve and scalar field (e.g., both BN254, or both BLS12-381). If they
drift — a circuit recompiled against a different curve, a verifier left on a default
config — proofs either fail to verify (availability loss) or, in the worse case,
verify against a weaker/attacker-favorable parameterization the deployer never
intended.

**Mainnet impact:** Critical when exploitable (proof-system-dependent; can range from
"contract is permanently bricked" to "verifier accepts proofs it shouldn't"), and the
failure mode is silent — nothing in the Rust source signals a mismatch, since curve
and field parameters are usually configured independently in the two toolchains.

**Real-world precedent:** Curve/parameter mismatches between a circuit's declared
proving system and a verifier's hardcoded pairing parameters are a recurring
finding class in third-party Groth16-verifier audits, precisely because the two
artifacts are compiled by different toolchains with no shared source of truth.

**Mitigation:** [Z008](rules/Z008.md) — cross-checks curve/field identifiers declared
in the circuit source against the curve type used in the Rust verifier
(`ark_bn254::Bn254` vs. a BLS12-381 circuit, etc.).

---

### T8 — Insufficient Batch Validation in Rollup State Transitions

**Threat:** A ZK-rollup-style `apply_batch(old_root, new_root, proof, ...)` entry
point that writes `new_root` without first asserting `old_root == current_stored_root`
lets a stale or attacker-chosen batch overwrite state built on a different (or empty)
base — effectively replaying or forking the rollup's history.

**Mainnet impact:** Critical. State-root manipulation invalidates every account
balance or note the rollup tracks; typically unrecoverable without a hard fork or
manual reconciliation.

**Real-world precedent:** The Nomad Bridge hack (August 2022, ~$190M) is not a ZK
protocol, but it is the canonical precedent for exactly this failure shape: a trusted
root was reinitialized to a value (`0x00`) that was accepted as "proven" by the
verification logic, letting anyone replay arbitrary messages against it. It is cited
here as the closest large-scale, public incident illustrating what happens when a
"verify against the current committed root" check is missing or degrades to
always-true.

**Mitigation:** [Z013](rules/Z013.md) — require `old_root == current_root` assertion
before accepting `new_root`, keeping the proof's claimed starting state anchored to
what the contract actually holds.

---

### T9 — Merkle-Root Spoofing (Membership Proofs)

**Threat:** A function that accepts a leaf and a Merkle path but computes the root
without comparing it against the contract's *stored* root (or worse, accepts a
caller-supplied root) will accept membership proofs for arbitrary attacker-chosen
trees.

**Mainnet impact:** Critical for any allowlist, UTXO set, or anonymity-set check —
an attacker proves membership in a tree they built themselves, bypassing the
membership requirement entirely.

**Real-world precedent:** Same 0xPARC-tracked bug family as T6; Merkle-membership
verifiers are a recurring target because the root comparison is easy to omit when the
rest of the verification logic (path recomputation) is present and "looks correct."

**Mitigation:** [Z014](rules/Z014.md) — load the root from contract storage, verify
the computed root against it, and nullify the leaf after use (combine with T3's
pattern).

---

### T10 — Predictable Secrets / Insecure Randomness as Circuit Input

**Threat:** Using on-chain predictable data (ledger timestamp, sequence number) as a
secret input to a commitment or nullifier construction lets an attacker predict or
brute-force it, breaking the privacy or uniqueness guarantee the ZK scheme exists to
provide.

**Mainnet impact:** High. Doesn't directly move funds, but collapses the privacy
property the contract advertises — commitments become linkable, defeating the point of
using ZK at all — and can enable griefing (front-running a predictable nullifier).

**Real-world precedent:** Weak/predictable entropy for secrets is a generic
cryptographic footgun, but it is specifically severe in ZK privacy protocols because
the entire threat model assumes the secret is unknown to anyone but the prover; this
is the same failure class documented for on-chain "randomness" in gambling/lottery
contracts, applied to commitment secrecy instead of unpredictability of outcome.

**Mitigation:** [Z002](rules/Z002.md) (`ZK_INSECURE_RANDOMNESS`) — flags ledger
timestamp/sequence used as a commitment or nullifier secret; secrets must be generated
off-chain by the prover with a secure RNG.

---

### T11 — Commitment-Scheme Reuse Without Domain Separation

**Threat:** If nullifiers, leaf commitments, and other semantically distinct values
are all hashed with the same function and no domain tag, a value legitimately produced
in one context can be replayed as if it belonged to another (e.g., a leaf commitment
submitted where a nullifier is expected).

**Mainnet impact:** Medium–High, context-dependent — enables cross-context collision
attacks that bypass whichever check assumed domain uniqueness.

**Real-world precedent:** Hash-domain confusion is a long-standing generic
cryptographic-protocol bug class (distinct from any single named ZK incident), and is
explicitly called out as a required pattern in this repo's own
[`zk-security-guide.md §2.5`](zk-security-guide.md#25-domain-separation).

**Mitigation:** [Z011](rules/Z011.md) — require a distinct, versioned domain-tag
prefix (`b"sanctifier:nullifier:v1"`, etc.) on every commitment-hash construction.

---

### T12 — Unbounded Proof-Verification Loops (DoS)

**Threat:** A batch-verification entry point that loops over a caller-supplied,
unbounded list of proofs can exceed Soroban's CPU instruction budget, making the
contract's core function unusable (or making it exploitable as a griefing vector
against other users sharing the same ledger resource budget).

**Mainnet impact:** High — availability loss, not fund loss directly, but can be used
to stall time-sensitive operations (e.g., a claim window) for legitimate users.

**Real-world precedent:** Resource-exhaustion-via-unbounded-loop is a generic
smart-contract DoS pattern; it is specifically acute for ZK batch verification because
each individual pairing check is already CPU-expensive relative to ordinary contract
operations, so the same loop-length bug bites far sooner than in non-ZK contracts.

**Mitigation:** [Z009](rules/Z009.md) — cap batch size at a protocol-defined constant
before entering the verification loop.

---

### T13 — Privacy Leak via Public-Output Over-Exposure

**Threat:** A circuit/contract that exposes more public outputs than strictly
necessary for verification leaks information the ZK scheme was meant to keep private
(e.g., publishing an intermediate value that narrows the private witness space).

**Mainnet impact:** Medium — privacy degradation rather than direct fund loss, but
undermines the core value proposition of using ZK, and can compound with T10 to make
formerly-private data inferable.

**Real-world precedent:** Over-broad public signals are a common circuit-design
mistake flagged in third-party circuit audits (distinct from any single named
incident) — the fix requires deliberately minimizing what's proven-public vs.
proven-private at circuit design time, which this rule flags as advisory rather than
a hard block.

**Mitigation:** [Z012](rules/Z012.md) — advisory/heuristic flag on public outputs
beyond what the entry point's stated purpose requires.

---

## 5. Threat-to-Rule Coverage Matrix

| Threat | Class | Severity | Rule(s) | Detector status |
|---|---|---|---|---|
| T1 — Trusted-setup compromise | zk-trusted-setup | Critical | [Z004](rules/Z004.md) | ✅ implemented |
| T2 — Proof malleability | zk-proof-integrity | Critical | [Z003](rules/Z003.md) | ✅ implemented |
| T3 — Nullifier/proof replay | zk-proof-integrity | Critical/High | [Z001](rules/Z001.md), [Z006](rules/Z006.md) | ✅ / ⏳ documented |
| T4 — VK integrity | zk-trusted-setup | High | [Z005](rules/Z005.md) | ✅ implemented |
| T5 — VK rotation access control | zk-access-control | Critical | [Z010](rules/Z010.md) | ✅ implemented |
| T6 — Under-constrained circuit | zk-circuit-constraints | Critical | [Z007](rules/Z007.md) | ◐ partial (arkworks only) |
| T7 — Curve/field confusion | zk-circuit-constraints | Critical | [Z008](rules/Z008.md) | ⏳ documented |
| T8 — Rollup batch-root validation | zk-proof-integrity | Critical | [Z013](rules/Z013.md) | ⏳ documented |
| T9 — Merkle-root spoofing | zk-proof-integrity | Critical | [Z014](rules/Z014.md) | ⏳ documented |
| T10 — Insecure randomness | zk-randomness | High | [Z002](rules/Z002.md) | ⏳ documented |
| T11 — Domain-separation failure | zk-cryptography | High | [Z011](rules/Z011.md) | ⏳ documented |
| T12 — Unbounded verification loop | zk-resource | High | [Z009](rules/Z009.md) | ⏳ documented |
| T13 — Public-output over-exposure | zk-privacy | Medium | [Z012](rules/Z012.md) | ⏳ documented |

✅ implemented and registered · ◐ partial · ⏳ specified, detector not yet landed
(statuses mirror [`zk-roadmap.md`](zk-roadmap.md#z-rule-catalogue) at time of writing).

**Critical rules** (Z001, Z003, Z004, Z007, Z008, Z010, Z013, Z014) must be resolved
before mainnet deployment, consistent with
[`zk-security-guide.md §3`](zk-security-guide.md#3-z-rule-catalog).

---

## 6. Gaps and Newly Proposed Mitigations

Three detectors already exist in the pipeline without a stable Z-code
([`zk-roadmap.md` — known gap #5](zk-roadmap.md#known-gaps-in-this-wave)). Each
corresponds to a threat this document identifies independently; assigning them codes
here closes that gap rather than leaving it implicit.

### T14 — Discarded Proof-Verification Result

**Threat:** `verify_proof(...)` is called but its return value is never checked
(assigned to `_`, or the function continues regardless of `Result::Err`). The proof
verification becomes decorative — the contract proceeds identically whether the proof
was valid or not.

**Mainnet impact:** Critical — functionally equivalent to having no verifier at all
for that entry point.

**Proposed mitigation:** **Z015 (proposed)** — maps to the existing uncoded detector
`zk_verification_result_ignored`. Require verification results to be propagated via
`?` or an explicit `assert!`/`require!`, as in the
[private-transfer pattern](case-studies/zk-private-transfer.md#3-proof-verification-result-handling).

### T15 — Verifier Reachable Only Through One Conditional Branch

**Threat:** A verifier call sits inside an `if`/`else` where the "effect" branch is
reachable independent of which arm executes, or the check can be routed around by a
caller-controlled flag — the proof is real, but not actually load-bearing for every
path that reaches the effect.

**Mainnet impact:** Critical — an attacker who finds the unverified branch bypasses
proof verification entirely.

**Proposed mitigation:** **Z016 (proposed)** — maps to the existing uncoded detector
`zk_verifier_skippable`. Require the state-changing effect to be dominated by the
verification call in the control-flow graph (no path reaches the effect without
passing through a passing verification).

### T16 — Empty-Bodied ZK Entry Point

**Threat:** An entry point that accepts a proof and public inputs but never actually
asserts or constrains anything — a stub or an incompletely wired verifier left in
production.

**Mainnet impact:** Critical — the entry point accepts *any* input as if it were
valid.

**Proposed mitigation:** **Z017 (proposed)** — maps to the existing uncoded detector
`zk_missing_constraint`. Flag ZK-tagged entry points with no assertion, no call into a
verifier function, and no constraint check anywhere in the body.

Numbering above is a proposal for this document's authors and the Z-rule maintainers
to ratify; it is not a claim that `Z015`–`Z017` are registered in
`data/vulnerability-db.json` or `finding_codes.rs` yet.

---

## 7. Non-Goals

- **Proof-system correctness.** This model does not attempt to verify that a
  hand-written pairing check correctly implements Groth16/PlonK arithmetic — it
  models how a verifier is *used* by contract logic, consistent with the roadmap's
  explicit scoping (["Deferred to a future wave"](zk-roadmap.md#deferred-to-a-future-wave)).
- **Ceremony replay.** Verifying that a cited trusted-setup ceremony transcript is
  itself valid requires network access and a full MPC verifier; Z004 checks that
  provenance is *cited*, not that the ceremony was honest.
- **Circuit soundness proofs.** Formally proving a circuit is not under-constrained is
  a substantially larger problem than the pattern-matching Z007 performs today; see
  the Z3-backed `S011` pass for the analogous (and separately scoped) contract
  invariant work.
- **Off-chain prover security.** Side-channel leakage, witness handling, and MPC
  ceremony execution happen entirely off-chain and outside what a Soroban contract
  analyzer can observe.
- **Sanctifier's own tooling surface.** Covered by `vscode-extension/THREAT_MODEL.md`
  (extension) and `docs/github-action-threat-model.md` (CI action), not repeated here.

---

## 8. Security Contact

Report vulnerabilities in contracts analyzed by these rules, or in the rules
themselves, via the
[HyperSafeD/Sanctifier GitHub Security Advisories](https://github.com/HyperSafeD/Sanctifier/security/advisories/new).

---

## 9. Further Reading

- [0xPARC ZK Bug Tracker](https://github.com/0xPARC/zk-bug-tracker)
- [Trail of Bits: Coordinated disclosure of vulnerabilities affecting Girault, Bulletproofs, and PlonK ("Frozen Heart")](https://blog.trailofbits.com/2022/04/13/part-1-coordinated-disclosure-of-vulnerabilities-affecting-girault-bulletproofs-and-plonk/)
- [Zellic: The Frozen Heart Vulnerability in PlonK](https://www.zellic.io/blog/the-frozen-heart-vulnerability-in-plonk)
- [`docs/zk-security-guide.md`](zk-security-guide.md) — secure design patterns per threat class
- [`docs/ZK-INTEGRATION-GUIDE.md`](ZK-INTEGRATION-GUIDE.md) — CI wiring for `sanctifier zk lint`
- [`docs/case-studies/zk-private-transfer.md`](case-studies/zk-private-transfer.md) — worked example of T3
- [`docs/adr/010-zk-feature-integration.md`](adr/010-zk-feature-integration.md) — architecture this threat model assumes
