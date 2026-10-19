# ADR 009: Target Soroban/WASM Rather Than EVM

## Status

Accepted

## Context

Sanctifier is a smart contract security tool. The two dominant smart-contract platforms
at the time of the project's founding were:

| Platform | VM | Primary language | Ecosystem maturity |
|----------|----|------------------|--------------------|
| **EVM** (Ethereum, Polygon, …) | Stack-based bytecode | Solidity / Vyper | High — Slither, Mythril, Echidna, Foundry, Certora |
| **Soroban** (Stellar) | WASM | Rust | Low — almost no tooling existed at mainnet launch (2024) |

We had to decide which platform to build for first.

The existing security tooling landscape was also a factor:

- EVM already has a mature ecosystem of open-source analyzers (Slither, Mythril),
  fuzz frameworks (Echidna, Foundry), and formal verifiers (Certora, Halmos).
- Soroban launched to mainnet in 2024 with none of that scaffolding. Every team was
  re-inventing the same review checklist from scratch.

## Decision

We target **Soroban** (the Stellar smart-contract platform) with **WebAssembly (WASM)**
as the compile target.

Specifically:
- Analysis rules operate on Rust source code (the dominant language for Soroban contracts).
- The CLI is compiled to native binaries for distribution but also cross-compiles to
  `wasm32-unknown-unknown` for the browser/dashboard path.
- Rules are modelled around Soroban's specific security footguns: `require_auth`,
  storage TTL semantics, SEP-41 token interface, ledger-entry size limits, and
  WASM-specific integer overflow behaviour.

## Alternatives Considered

### EVM (Solidity/Vyper)

Building an EVM analyzer would provide a larger immediate addressable market. We did not
choose this path because:

- The EVM tooling space is crowded with well-funded, established projects. Sanctifier
  would be a marginal improvement over Slither or Mythril, not a step-change.
- Soroban was an unserved market: zero open-source security analyzers existed at launch.
  First-mover advantage and ecosystem impact are significantly higher.
- The team has deep Rust expertise. Analyzing Rust source with `syn` and Z3 is a
  natural fit; Solidity analysis would require building or integrating a Solidity parser.

### Multi-chain from day one

Supporting both EVM and Soroban simultaneously would have halved the depth of analysis
on each platform. We chose to go deep on one platform first and revisit EVM support
once the Soroban rule set is mature.

### CosmWasm (Cosmos)

CosmWasm contracts are also written in Rust and compiled to WASM, making the tech stack
similar. We did not target CosmWasm first because Soroban's authorization model
(`require_auth`) and ledger semantics differ significantly from CosmWasm's message-
passing model, and Stellar's marketing push around Soroban created a larger immediate
community of developers who needed tooling.

## Consequences

**Positive:**
- Sanctifier addresses a genuine gap: the only open-source security analyzer for Soroban
  at mainnet launch.
- WASM as a compilation target means the analysis engine can run in the browser
  (dashboard) and in VS Code (via the extension host), using the same binary.
- Rust's type system and `syn` parsing make AST-level rules straightforward to write
  and test without a custom language front-end.

**Negative:**
- The Soroban SDK is evolving rapidly. Rules tied to specific API signatures (e.g.
  `storage().persistent().set(...)`) require updates as the SDK changes.
- The user base is smaller than EVM. Early contributor growth may be slower.
- WASM integers behave differently from EVM 256-bit integers; overflow rules must
  account for `i128`/`u128` wrap-around on the `wasm32` target rather than Solidity's
  overflow revert behaviour.
