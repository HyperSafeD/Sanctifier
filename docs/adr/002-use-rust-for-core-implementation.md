# ADR 002: Use Rust for Core Implementation

## Status

Accepted

## Context

Sanctifier is designed to be a high-performance, secure static analysis tool specifically targeting Smart Contracts written for the Stellar (Soroban) network. We need a language that can parse WebAssembly (WASM) efficiently, manipulate complex Abstract Syntax Trees (ASTs), run quickly as a CLI tool, and potentially be compiled to WASM itself for browser-based execution in the future.

The primary language for writing Soroban smart contracts is Rust. Building our tooling in the same language ecosystem offers synergies in parsing and understanding the host contracts.

## Decision

We will use **Rust** as the primary programming language for the core Sanctifier implementation (analyzers, CLI, and core logic).

## Consequences

**Positive:**
* **Performance:** Rust is a compiled language with no garbage collector, offering excellent performance which is crucial when analyzing large WASM binaries or deeply nested ASTs.
* **Safety:** Rust's ownership model prevents many classes of memory safety vulnerabilities, leading to a more robust and secure tool.
* **Ecosystem Compatibility:** Since Soroban contracts are written in Rust, we can leverage the same crates (e.g., for parsing Rust syntax or interacting with Soroban SDK constructs) if we extend our analysis to source code.
* **WASM Support:** Rust has first-class support for compiling to WebAssembly, keeping the door open for web-based versions of Sanctifier without a full rewrite.

**Negative:**
* **Learning Curve:** Rust's borrow checker and lifetimes can present a steep learning curve for new contributors compared to a language like TypeScript or Python.
* **Compilation Time:** Compiling large Rust applications can be slow, slightly impacting the local development loop.
