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
- The released binaries carry no Z3 runtime dependency: the release workflow builds
  `-p sanctifier-cli`, which compiles `sanctifier-core` without `smt`. (That workflow still installs
  Z3 before building; the CLI build does not use it.)
- The release workflow builds Linux x86_64 (glibc and musl), macOS x86_64 and arm64, and Windows
  x86_64. There is **no Windows arm64 build**.
- If you build a variant with the `smt` feature enabled, Z3 must be dynamically or statically
  linked; for static linking, follow the `z3-sys` static compilation instructions.

## Release channels

Every channel is driven by pushing a `v*` tag. Three separate workflows are involved:

| Channel | Workflow / job | Notes |
|---------|----------------|-------|
| GitHub release binaries and `.sha256` files | `release.yml` | Tags containing `-mainnet` are first checked against the open issues in the *Mainnet Launch Readiness* milestone. |
| Docker, `ghcr.io/hypersafed/sanctifier` | `release.yml` (`docker`) | Built from the musl binary. |
| npm, `@hypersafed/sanctifier-cli` | `release.yml` (`npm-publish`) | Needs the `NPM_TOKEN` secret. The package downloads the release binary on first run. |
| Homebrew, `HyperSafeD/sanctifier/sanctifier` | `release.yml` (`homebrew-update`) | Rewrites `homebrew/sanctifier.rb` and pushes it to the `HyperSafeD/homebrew-sanctifier` tap. **Skipped** (with a log message) when the `HOMEBREW_TAP_TOKEN` secret is not set. |
| crates.io, `cargo install sanctifier-cli` | `publish.yml` | Publishes `sanctifier-core` first, waits for it to appear on crates.io, then publishes `sanctifier-cli`. Needs `CRATES_IO_TOKEN`. |
| Scoop | `winget-scoop.yml` | Regenerates `scoop/sanctifier.json` and commits it to this repository. |
| winget | `winget-scoop.yml` | Updates the manifests under `winget/` and opens a pull request against **this repository's** `main` (the step is `continue-on-error`). It does **not** submit to `microsoft/winget-pkgs`, so `winget install HyperSafeD.Sanctifier` only works once someone has submitted those manifests there. |

Known gap: the generated Scoop manifest also lists a `sanctifier-windows-arm64.exe` download (reusing
the x86_64 hash), but the release workflow does not build that asset.

### Release status (checked 2026-10-19)

No `v*` tag has been pushed yet and none of these workflows has run, so **the channels above are not
populated yet**: `sanctifier-cli` and `sanctifier-core` are not on crates.io, `@hypersafed/sanctifier-cli`
is not on npm, there are no GitHub releases, and `microsoft/winget-pkgs` has no `HyperSafeD`
manifests. (The Homebrew tap repository could not be found from an account outside the
organisation.) Until the first release, build from source as described above. To check the current
state yourself:

```sh
gh release list -R HyperSafeD/Sanctifier
cargo search sanctifier-cli
npm view @hypersafed/sanctifier-cli version
```

## Installation via Cargo

Once a release has been published to crates.io, you can install the CLI with:
```sh
cargo install sanctifier-cli
```

Before that, build from a checkout of this repository as described in
[Building from Source](#building-from-source).

The other channels in the table above are listed, with commands, in the
[README install options](../README.md#installation-methods).

## Running Backwards Compatibility Tests

We maintain backwards compatibility for standard output and flags. Run the versioning and compatibility suite via:
```sh
cargo test -p sanctifier-cli --test versioning_tests
```
