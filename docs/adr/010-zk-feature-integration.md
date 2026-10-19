# ADR 010: ZK Feature Integration — Motivation and High-Level Architecture

## Status

Proposed

## Context

Sanctifier is a static/runtime security analyzer for Soroban smart contracts (see
[ADR 009](009-soroban-wasm-target.md)), built around a Rust-source pipeline: `syn`-based
parsing ([ADR 007](007-syn-ast-parser.md)), a rule registry emitting `S0xx` findings
([`finding_codes.rs`](../../tooling/sanctifier-core/src/finding_codes.rs)), optional Z3
formal verification ([ADR 006](006-z3-formal-verification.md)), and SARIF output
([ADR 008](008-sarif-output-format.md)) consumed by the CLI, CI, the VS Code extension,
and the dashboard.

Zero-knowledge (ZK) contracts are an increasingly common pattern on Soroban — private
transfers, ZK rollup examples, and Groth16-style verifier contracts. They introduce a
class of bug that Sanctifier's existing `S0xx` rule set cannot see: missing nullifier
checks, malformed or forged proof verification, unconstrained circuit signals, and
trusted-setup handling — none of which show up as ordinary Rust control-flow or
authorization footguns. At the same time, the *source* of ZK logic is not always Rust:
circuits are commonly authored in circom or Noir, languages `syn` cannot parse.

Without a deliberate integration plan, ZK support risks becoming a bolt-on: a
special-cased parser here, an ad hoc severity guess there, findings that don't survive
the SARIF pipeline other surfaces depend on. This ADR is the capstone document for that
integration — it exists so a future contributor can understand *why* Sanctifier added
ZK support and *how* the pieces fit together, in one place, instead of reconstructing it
from the ~60 individual issues that make up the ZK wave.

This ADR does not re-decide anything settled elsewhere. It summarizes and links to the
sub-decisions:

- **[#1191](https://github.com/HyperSafeD/Sanctifier/issues/1191) — ZK toolchain
  selection.** Which circuit toolchain(s) (circom+snarkjs, arkworks, halo2, Noir)
  Sanctifier supports first-class, and which are later-roadmap. Recorded as its own ADR
  once merged (expected `docs/adr/011-zk-toolchain-selection.md` or similar, numbering
  TBD at merge time).
- **[#1192](https://github.com/HyperSafeD/Sanctifier/issues/1192) — `Z`-series
  namespace and severity taxonomy.** How ZK findings are code-numbered (`Z001`–`Z0NN`,
  parallel to `S0xx`) and how they map onto
  [`schemas/severity-taxonomy.schema.json`](../../schemas/severity-taxonomy.schema.json).
- **[#1194](https://github.com/HyperSafeD/Sanctifier/issues/1194) — pipeline design
  doc.** The detailed design of how new parser front-ends plug into
  `sanctifier-core` without destabilizing the existing `S`-rule pipeline.

As of this writing, #1191, #1192, and #1194 are still open — this ADR describes the
target shape agreed in those issues' own scope sections, and should be read as
"direction of travel" for anything not yet merged. Update the links above once each
lands.

## Decision

Sanctifier will add ZK support as a **parallel front-end, shared back-end** extension
of the existing pipeline, not a separate tool:

```
circom / Noir / arkworks-Rust source
              │
              ▼
   new ZK parser front-ends            existing syn-based Rust parser
   (circom, Noir — new crates/modules) (unchanged, S-rules keep using it)
              │                                       │
              └───────────────┬───────────────────────┘
                               ▼
                  shared intermediate Finding representation
                               │
                               ▼
                  rule registry: S-rules (existing) + Z-rules (new)
                               │
                               ▼
                  existing severity mapping / SARIF output layer
                               │
                               ▼
              CLI · CI · VS Code extension · dashboard (unchanged)
```

Concretely:

1. **New parser front-ends, not a new pipeline.** circom and Noir source get their own
   parsing crates/modules (scope of #1227, #1228); Rust-embedded circuit code
   (arkworks) reuses the existing `syn` parser (#1229) rather than adding a third
   front-end. Every front-end still terminates in the same `Finding` type the `S`-rules
   already produce — see #1194 for the detailed plug-in design.
2. **`Z`-rules are first-class citizens of the existing rule registry**, not a bolt-on
   post-processing step. They are numbered `Z001`–`Z0NN` per #1192 and registered
   alongside `S0xx` rules so a single analysis run can report both.
3. **No changes to the SARIF/severity output layer are assumed by default.** Per
   #1194's scope, the existing `Finding` → SARIF path should absorb ZK findings without
   a breaking schema change; if #1192's severity work finds it needs an extension,
   that's additive, not a fork of the output format.
4. **Untrusted circuit input is treated as an attack surface on the tool itself**,
   consistent with how this project already threat-models its own tooling (see
   `vscode-extension/THREAT_MODEL.md`, `docs/github-action-threat-model.md`). The
   parser-specific threat model, fuzzing, and mainnet-readiness security review are
   tracked separately (see Related Work below) rather than folded into this ADR.
5. **The feature ships behind a flag** until the reference verifier, rollup example,
   and Z-rule set have been reviewed, consistent with how this project gates other
   high-risk surfaces before general availability.

## Alternatives Considered

### Separate standalone tool for ZK analysis

Building a dedicated `sanctifier-zk` binary with its own output format would avoid any
risk of destabilizing the existing `S`-rule pipeline. Rejected because it would
duplicate the SARIF/severity/CI/IDE integration work Sanctifier has already built and
force users to run and correlate two tools instead of one `sanctifier check`.

### Treat ZK circuits as opaque and only analyze the surrounding Rust glue code

Only analyzing the Rust verifier-contract wrapper (which Sanctifier can already parse)
and ignoring circuit source entirely would require zero new parsers. Rejected because
the highest-severity ZK bugs (missing constraints, forged proofs, malformed circuits)
live inside the circuit definition itself, not the wrapper — skipping it would make the
feature security theater.

### Fork the output pipeline for ZK-specific findings

A ZK-specific SARIF-like format tailored to circuit findings (e.g. constraint-graph
annotations) was considered. Rejected for the same reason as the standalone-tool option:
it would break the "every surface consumes one SARIF stream" property established in
[ADR 008](008-sarif-output-format.md), for a benefit (richer circuit-specific metadata)
that can instead be carried in SARIF's existing `properties` bag.

## Consequences

**Positive:**

- Users get ZK findings through the same CLI, CI gate, VS Code panel, and dashboard
  they already use — no second tool, no second report to reconcile.
- The `S`-rule pipeline is architecturally insulated from ZK-specific churn: new parser
  front-ends are additive, and `Z`-rules live in their own numbering space.
- Treating parser input as untrusted from the start (rather than retrofitting it) means
  fuzzing and threat-modeling can be sequenced *before* the dashboard upload path goes
  live, not after an incident.

**Negative:**

- Two new source languages (circom, Noir) mean two new parsers to maintain, each with
  its own edge cases and evolving grammar — ongoing maintenance surface the `S`-rule
  side doesn't have.
- The shared `Finding`/SARIF representation may prove too narrow for some circuit-level
  findings (e.g. constraint-graph visualizations) and need a future additive extension.
- ZK cryptography bugs (circuit soundness, trusted-setup handling) require reviewer
  expertise most of Sanctifier's existing contributor base doesn't have; the tool can
  flag known anti-patterns but is not a substitute for specialist review of the
  reference contracts it ships.

## Related Work

This ADR covers motivation and high-level architecture only. Related, separately
tracked work that a future reader may be looking for:

- **#1196 — ZK feature roadmap.** What ships in this wave vs. deferred scope.
- **#1248 — ZK tooling threat model.** Threats specific to processing untrusted
  circuit input (parser crashes, resource exhaustion), distinct from #1193's
  threat model of the ZK-verifier *contracts* themselves.
- **#1249 — Fuzzing harnesses** for the circom and Noir parsers, addressing the
  parser-crash risk identified in the tooling threat model above.
- **#1251 — ZK-specialist security review.** A dedicated cryptography-focused review
  of the reference verifier, rollup example, and Z-rule detection logic for
  false-negative risk, gating removal of the feature flag for mainnet users —
  distinct from, and in addition to, the general contract audit in #1112.

None of the above are implemented by this ADR; they are linked here so the "why and how
ZK support was added" story is discoverable from one place, per this issue's goal.
