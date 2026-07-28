# ADR 006: Use Z3 for Formal Verification

## Status

Accepted

## Context

Sanctifier's static heuristics (rules S001–S012) catch common coding mistakes by
pattern-matching source text and an AST. They produce false-positive-free results for
known footguns, but cannot make exhaustive mathematical guarantees — they cannot prove
the *absence* of a bug, only flag likely occurrences.

To offer a stronger correctness story — one that auditors and protocol teams can rely
on in production — we need a formal verification backend capable of disproving invariants
(e.g. "this function cannot be called without authentication") across all possible
program states.

The main candidates evaluated were:

| Tool | Approach | Soroban fit |
|------|----------|-------------|
| **Z3** (Microsoft Research) | SMT solver, programmable via API | High — rich integer and bitvector theories match Soroban's `i128`/`u128` types |
| **Kani** | Bounded model-checker for Rust, LLVM-based | Medium — catches panics/overflows in unit-test style harnesses |
| **Creusot** | Deductive verifier; requires Rust source annotations | Low — requires contract authors to write `#[requires]`/`#[ensures]` attributes |
| **Prusti** | Viper-based Rust verifier; annotation-heavy | Low — same annotation burden as Creusot |

## Decision

We use **Z3** (via the `z3` Rust crate) as the SMT backend for Sanctifier's formal
verification layer (rule S011).

Sanctifier translates a subset of Soroban contract logic into Z3 assertions and calls
the solver to check satisfiability. An `unsat` result proves that no counter-example
exists; a `sat` result with a model surfaces the concrete input that violates the
invariant.

## Alternatives Considered

### Kani

Kani is an excellent choice for bounded checking of individual Rust functions and would
catch many overflow and panic paths. We did not choose it as the primary formal backend
because:

- Kani operates on LLVM IR, not on a structured IR we control, making it harder to map
  findings back to Soroban-specific semantic concepts (authorization model, ledger keys).
- Kani requires the full Rust toolchain at analysis time; Z3 can be embedded as a static
  or dynamic library and shipped with the Sanctifier binary.
- Kani is a point-in-time verifier (bounded unwind depth), whereas Z3 can prove
  unbounded properties over abstract state.

Kani integration is tracked separately in `docs/kani-integration.md` as a complementary
tool, not a replacement.

### Creusot / Prusti

Both require contract authors to annotate their source with pre- and post-conditions.
Sanctifier's design goal is *zero-annotation* analysis — drop the binary on any Soroban
crate and get findings without modifying the source. Creusot and Prusti cannot meet this
requirement without significant annotation scaffolding.

## Extensions

### Circuit range-check verification (Z007 deep-verify, #1213)

The SMT layer has been extended to encode Circom circuit constraints as Z3
assertions (`smt::circuit_range`).  For each signal used in a comparison
(`<`, `>`, `<=`, `>=`), the solver checks whether the signal is provably
bounded by the accumulated constraint set — beyond what the heuristic AST
pattern in Z007 can detect.

**Encoding approach:**
- Each signal is modelled as a `Z3 Int` variable constrained to `[0, p-1]`
  where `p` is the BN254 field modulus.
- Linear constraints (`===`) are translated directly as equality assertions.
- Comparison expressions (`a < b`) are modelled as `ite(a < b, 1, 0)`.
- The solver is asked whether the signal can exceed a reasonable bound
  (e.g. 2^64 - 1) while still satisfying all constraints — `Sat` means
  the signal is under-constrained.

**Limitations** (documented in `circuit_range.rs` and ADR-011):
- Only BN254 field modulus is supported.
- Component instantiations are not inlined.
- `Num2Bits` / bit-vector range checks are not yet modelled.
- The encoding is a conservative approximation — missed constraints may
  produce false positives.

## Consequences

**Positive:**
- S011 findings carry mathematical weight: a failing assertion is a *proof* that the
  invariant is violated for the shown input.
- Z3's bitvector and integer theories align naturally with Soroban's `i128`/`u128` token
  arithmetic.
- The `z3` crate is actively maintained and licensed MIT.
- Circuit range-check verification (Z007 deep-verify) adds a formally-grounded
  complement to heuristic pattern matching for under-constrained signal detection.

**Negative:**
- Z3 is a heavyweight dependency (~30 MB shared library). The Sanctifier binary ships
  with Z3 statically linked, increasing the binary size.
- Only a subset of Soroban contract semantics can be encoded in Z3's first-order logic;
  very complex control flow requires abstraction that may miss some paths.
- SMT solving is NP-hard in the worst case; pathological contracts can time out. We
  apply a configurable solver timeout (default: 5 s) and emit a warning when it fires.
- Circuit range-check encoding does not model component instantiations or bit-vector
  range checks; see ADR-011 for the full scope of formal-verification limitations.
