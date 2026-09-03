# ADR 007: Use syn for Rust AST Parsing

## Status

Accepted

## Context

Sanctifier's analysis rules operate on structured representations of Soroban contract
source code. To flag patterns like "a `pub fn` inside `#[contractimpl]` that writes to
storage without calling `require_auth`" we need access to the syntactic and semantic
structure of the Rust source — not just raw text.

Three parsing approaches were evaluated:

| Approach | Description | Fit |
|----------|-------------|-----|
| **`syn`** | Rust proc-macro parser; produces a full Rust AST | High — native Rust types, actively used by the Rust community |
| **tree-sitter** | Incremental, language-agnostic PEG parser | Medium — good for editors; grammar must be kept in sync with Rust edition |
| **Custom parser** | Hand-written recursive descent | Low — enormous maintenance surface; no benefit over `syn` |
| **rustc API** | Direct access to HIR/MIR via compiler internals | Low — unstable API, requires nightly, complex setup |

## Decision

We use **`syn`** to parse Rust source files into a typed AST.

Rules traverse the `syn::File` → `syn::Item` → `syn::ImplItem` hierarchy directly,
using Rust pattern matching. Attribute detection (e.g. `#[contractimpl]`) uses
`syn::Attribute` helpers. Token-level context (e.g. line numbers for diagnostics) is
recovered via `proc_macro2::Span`.

## Alternatives Considered

### tree-sitter

tree-sitter produces concrete syntax trees incrementally and is language-agnostic, making
it the preferred choice for editors and language servers. We did not choose it because:

- The `tree-sitter-rust` grammar tracks stable Rust but lags behind new editions.
  `syn` is the canonical Rust parser and is updated in lockstep with each language
  edition by the Rust team.
- tree-sitter CSTs expose raw grammar nodes (strings like `"function_item"`), not
  typed Rust AST nodes. Writing rules against typed `syn` enums is safer and
  easier to maintain than matching string node names.
- Sanctifier does not need incremental parsing — it re-parses the whole file on each
  analysis pass, which `syn` handles in microseconds for typical contract files.

### Custom parser

Writing a bespoke Rust parser would duplicate `syn`'s extensive test coverage and battle-
hardened handling of Rust's complex grammar (lifetimes, where clauses, macros, etc.).
The maintenance burden would be prohibitive for an open-source project.

### rustc API (HIR/MIR)

Accessing the Rust compiler's internal representations (HIR for name-resolved trees,
MIR for control flow) would enable more powerful analyses (type information, monomorphized
call graphs). We plan to explore this for a future "deep analysis" mode, but it is
unsuitable as the primary parsing layer because:

- The `rustc_*` crates require nightly Rust and have no stability guarantees.
- Users would need to run Sanctifier via `cargo +nightly`, which conflicts with the
  goal of zero-configuration drop-in analysis.

## Consequences

**Positive:**
- Rules are written as idiomatic Rust pattern matches against typed AST nodes — easy
  to review, test in isolation, and extend.
- `syn` handles all Rust editions and macro expansion at the token level.
- `syn` is already a transitive dependency of many Rust projects; it adds no new
  supply-chain risk.

**Negative:**
- `syn` parses source text, not type-checked code. Rules cannot see resolved types,
  inferred lifetimes, or generic instantiations. Type-sensitive checks (e.g. detecting
  that a value is `Address` specifically) rely on naming conventions rather than type
  inference.
- Macro-generated code is opaque: if a user writes a procedural macro that emits
  storage writes, Sanctifier will not see them.
