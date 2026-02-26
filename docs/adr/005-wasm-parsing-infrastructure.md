# ADR 005: Use Walrus for WASM Parsing

## Status

Accepted

## Context

To perform static analysis on compiled Soroban contracts, Sanctifier must parse the WebAssembly binaries. We need a library that provides a high-level, mutable, and navigable representation of WASM modules. We don't just want to interpret bytes; we need structured access to functions, local variables, imports, exports, and individual instructions to track data flow and control flow.

Several options exist in the Rust ecosystem:
1. `wasmparser`: Very fast, zero-allocation, iterator-based. Good for simple validation but hard to perform complex global analysis on since it doesn't build a full AST.
2. `parity-wasm`: An older standard, somewhat deprecated, manipulates ASTs.
3. `walrus`: Built specifically by the Rust/WASM working group for manipulating and analyzing WASM modules. It builds a full graph of the WASM binary.

## Decision

We will use the **`walrus`** crate for parsing and analyzing WebAssembly binaries in Sanctifier.

## Consequences

**Positive:**
* **Rich Representation:** `walrus` builds a comprehensive, analyzable graph of the WASM module. It abstracts away raw indices in favor of typed IDs.
* **Manipulation:** It is designed from the ground up for analyzing and transforming WASM, which perfectly fits our needs for tracing execution paths or identifying unsafe instruction sequences.
* **Ecosystem:** It is a well-maintained, standard tool in the Rust/WASM ecosystem.

**Negative:**
* **Memory Overhead:** Building a full graph for very large WASM binaries will consume more memory than a streaming parser like `wasmparser`.
* **Complexity:** Navigating the `walrus` IR graph taking ownership and borrowing rules into account can be complex compared to simpler byte-matching approaches.
