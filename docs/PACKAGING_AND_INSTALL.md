# Packaging and Installation Guide

## Building from Source

To build the `sanctifier-cli` from source:

1. Ensure you have **Rust 1.88 or newer** installed.
2. Run `cargo build --release -p sanctifier-cli`.
3. The compiled binary will be available at `target/release/sanctifier`.

> **About the minimum Rust version.** The workspace manifest still declares
> `rust-version = "1.78"` (`[workspace.package]` in `Cargo.toml`), but that
> number is out of date and is not what limits you. With the committed
> `Cargo.lock`, several dependencies in the CLI's build (for example `time`,
> `darling`, `textwrap` and the `icu_*` crates) declare `rust-version = 1.88`,
> and some use edition 2024. Rust 1.87 stops with
> `rustc 1.87.0 is not supported by the following packages`, and Rust 1.78 fails
> earlier still while parsing a dependency manifest. No CI job builds with the
> declared minimum, and no member crate inherits the workspace value, so nothing
> flags this drifting.

The CLI depends on `sanctifier-core` with `default-features = false, features = ["parallel"]`, so
**building it does not require Z3 or libz3**. It does still need a **C compiler**: the TLS stack
(`reqwest` with `rustls`, which uses `ring`) compiles C and assembly during the build. libz3 is only
needed when you build the whole workspace (`make build`, `make release`, `cargo test --workspace`),
because `sanctifier-core` is then compiled with its default `smt` feature. See the
[README System Requirements](../README.md#system-requirements) for the per-platform packages.

### `sanctifier-core` feature flags

| Feature | Default | Effect on packaging |
|---------|---------|---------------------|
| `smt` | on | Links Z3 for rule S011. Off in the CLI; drop it for wasm32 targets. |
| `soroban` | on | Pulls in `soroban-sdk`. Off in the CLI and the wasm build. |
| `parallel` | on | Pulls in `rayon` for the batch analysis APIs. On in the CLI, off for wasm32 (no threads). |

## Distribution

When packaging `sanctifier` for distribution, note the following:
- The released binaries carry no Z3 runtime dependency.
- If you build a variant with the `smt` feature enabled, Z3 must be dynamically or statically
  linked; for static linking, follow the `z3-sys` static compilation instructions.

## Installation via Cargo

You can install the CLI directly from crates.io:
```sh
cargo install sanctifier-cli
```

Other published channels — npm/npx, Homebrew, Scoop, winget, Docker — are listed in the
[README install options](../README.md#installation-methods) and are updated automatically by the
release workflow on each tagged version.

## Running Backwards Compatibility Tests

We maintain backwards compatibility for standard output and flags. Run the versioning and compatibility suite via:
```sh
cargo test -p sanctifier-cli --test versioning_tests
```
