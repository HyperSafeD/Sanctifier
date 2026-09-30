# Fuzz-Harness Generator (`sanctifier harness`)

## Overview

`sanctifier harness` bridges Sanctifier's *static* analysis (AST-level
contract/ABI extraction) to *dynamic* analysis by generating ready-to-build
native fuzz-target scaffolds for a Soroban contract source file — no manual
harness-writing required.

Given a contract source file, the command:

1. Parses the file and discovers every `#[contractimpl]` block
   ([`sanctifier_core::harness_spec`](../tooling/sanctifier-core/src/harness_spec.rs)).
2. For each public, non-reserved function (i.e. excluding `__constructor` and
   `__check_auth`), extracts its typed parameter list (skipping the
   mandatory leading `Env` parameter).
3. Emits one fuzz target per function for each requested backend
   (`afl.rs`, `honggfuzz`, or both), plus a self-contained `Cargo.toml` per
   backend.

This targets the same invariant class as the hand-written harnesses
described in [`docs/contracts-fuzz.md`](contracts-fuzz.md) — "does this
contract entry point ever panic on attacker-controlled input?" — but removes
the need to hand-write a harness for every function.

## Usage

```bash
sanctifier harness path/to/contract.rs \
  --output fuzz-harness \
  --target both            # afl | honggfuzz | both (default)
  --function transfer      # optional: restrict to one function
```

This produces:

```
fuzz-harness/
├── afl/
│   ├── Cargo.toml
│   └── src/bin/
│       ├── token_transfer.rs
│       └── token_balance.rs
└── honggfuzz/
    ├── Cargo.toml
    └── src/bin/
        ├── token_transfer.rs
        └── token_balance.rs
```

Each `afl/` or `honggfuzz/` directory is an independent, workspace-excluded
Cargo package (`[workspace]` with no members — the same convention already
used by `contracts/my-contract/fuzz/Cargo.toml`), so it can be built without
disturbing the analyzed contract's own workspace:

```bash
cd fuzz-harness/afl
cargo afl build
cargo afl fuzz -i in -o out target/debug/token_transfer
```

```bash
cd fuzz-harness/honggfuzz
cargo hfuzz build
cargo hfuzz run token_transfer
```

## How inputs are generated

Every generated target follows the pattern documented by
[`soroban_sdk::testutils::arbitrary`](https://docs.rs/soroban-sdk/latest/soroban_sdk/testutils/arbitrary/index.html)
for fuzzing host-managed contract types: each parameter becomes a
`<ParamType as SorobanArbitrary>::Prototype` field on a derived `Arbitrary`
struct, and the harness body converts each prototype into its real Soroban
value with `.into_val(&env)` before calling `client.try_<function>(..)`. For
example, `transfer(env: Env, from: Address, to: Address, amount: i128)`
generates:

```rust
#[derive(Debug, Arbitrary)]
struct FuzzInput {
    from: <Address as SorobanArbitrary>::Prototype,
    to: <Address as SorobanArbitrary>::Prototype,
    amount: <i128 as SorobanArbitrary>::Prototype,
}

fn main() {
    fuzz!(|input: FuzzInput| {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, Token);
        let client = TokenClient::new(&env, &contract_id);

        let from: Address = input.from.into_val(&env);
        let to: Address = input.to.into_val(&env);
        let amount: i128 = input.amount.into_val(&env);

        let _ = client.try_transfer(&from, &to, &amount);
    });
}
```

`try_<function>` (rather than `<function>`) is used so the fuzzer treats
unexpected host-level panics as crashes while expected `Err` returns (e.g.
validation rejections) do not themselves count as findings; switch to the
non-`try_` call if you want to fuzz for panics *and* logic-level error paths.

Custom `#[contracttype]` structs/enums used as parameters are fuzzable too:
the Soroban SDK derives `SorobanArbitrary` for them automatically whenever
the `testutils` feature is enabled, which is why every generated `Cargo.toml`
enables `soroban-sdk`'s `testutils` feature.

## Crate auto-detection

`sanctifier harness` walks upward from the source file looking for the
nearest `Cargo.toml` to discover the contract crate's package name, and adds
it as a `path` dependency (with `features = ["testutils"]`) in the generated
manifest, plus the matching `use <crate>::{Contract, ContractClient};` in
each harness file. If no manifest is found, both are left as a `// TODO`
placeholder for you to fill in.

> **Note:** the target contract crate must itself define (or you must add) a
> `testutils` feature that turns on `soroban-sdk/testutils` — this is what
> makes `SorobanArbitrary` available for the crate's own `#[contracttype]`
> types. See the `soroban_sdk::testutils::arbitrary` module docs for details.

## Relationship to existing fuzz infrastructure

This command is a *generator*: it produces new, disposable scaffold crates
next to a contract you're analyzing. It does not replace or modify the
hand-written, CI-integrated harnesses described in
[`docs/contracts-fuzz.md`](contracts-fuzz.md), which continue to run in
`.github/workflows/contracts-fuzz.yml` as before.
