# ADR 004: Plugin System Design for Analyzers

## Status

Accepted

## Context

Sanctifier needs to detect a wide variety of vulnerabilities and anti-patterns in Soroban smart contracts. The rules for these detections will inevitably grow and evolve over time as new vulnerabilities are discovered. 

If all analysis logic is tightly coupled into a single massive matching engine, the codebase will become difficult to maintain, test, and extend. We need a way to encapsulate individual detection rules (analyzers) so they can be written independently, activated/deactivated via configuration, and eventually, enable community members to contribute their own rules.

## Decision

We will implement a **Plugin System** (Internal Module Registry pattern) for our vulnerability analyzers.

1. **Analyzer Trait**: All analyzers must implement a common `Analyzer` trait (or standard interface) which provides a unified entry point, taking the parsed WASM AST or metadata as input and returning a vector of `Vulnerability` structs.
2. **Registry**: There will be a central registry where analyzers are registered at compile-time (using Rust macros or explicit initialization functions).
3. **Execution Engine**: The core engine will iterate over all active analyzers in the registry and pipe the parsed contract data to them.

For the initial versions, these "plugins" will be statically linked internal modules rather than dynamically loaded external `.so` libraries, to maintain execution speed and cross-platform simplicity.

## Consequences

**Positive:**
* **Modularity:** Each vulnerability rule lives in its own isolated module, making it easy to test and maintain.
* **Extensibility:** Adding a new rule is as simple as creating a new struct implementing the `Analyzer` trait and registering it.
* **Configurations:** The engine can easily enable or disable specific plugins based on user configuration files (e.g., standard vs strict mode).

**Negative:**
* **Boilerplate:** Requires some boilerplate wiring to instantiate and register each new analyzer.
* **Shared State Overhead:** Analyzers must share read-only access to the parsed AST. If one analyzer needs specialized data extraction, we might end up doing redundant AST traversals unless we build a very sophisticated intermediate representation (IR) first.
