# Release Checklist

This document describes the steps required to publish a new release of Sanctifier.

## Pre-release

- [ ] Ensure all CI checks pass on `main` (build, test, lint, coverage)
- [ ] Update version in all `Cargo.toml` files to match the target release tag:
  - `Cargo.toml` (workspace)
  - `tooling/sanctifier-cli/Cargo.toml`
  - `tooling/sanctifier-core/Cargo.toml`
  - `tooling/sanctifier-detector/Cargo.toml`
  - `tooling/sanctifier-wasm/Cargo.toml`
- [ ] Update version in `vscode-extension/package.json`
- [ ] Update `CHANGELOG.md` — move unreleased entries to a new version section
- [ ] Run `cargo publish --dry-run -p sanctifier-core && cargo publish --dry-run -p sanctifier-cli` and fix any warnings
- [ ] Run `cargo publish --dry-run -p sanctifier-detector` and fix any warnings
- [ ] Verify all documentation links in `README.md` and `docs/` are valid
- [ ] Check that `CARGO_REGISTRY_TOKEN` is set in GitHub Secrets

## Release

- [ ] Create and push an annotated tag: `git tag -a vX.Y.Z -m "Release vX.Y.Z" && git push origin vX.Y.Z`
- [ ] Wait for the `Release` workflow to build and attach binaries
- [ ] Wait for the `Publish to Crates.io` workflow to complete
- [ ] Wait for the `Publish API Documentation` workflow to deploy docs
- [ ] Wait for the `Release VS Code Extension` workflow to publish to VS Code Marketplace

## Post-release

- [ ] Verify `cargo install sanctifier-cli` installs the latest version
- [ ] Verify the GitHub release page shows correct binaries and SHA256SUMS
- [ ] Verify https://crates.io/crates/sanctifier-cli shows the new version
- [ ] Verify `winget install HyperSafeD.Sanctifier` works
- [ ] Verify `scoop install sanctifier` works
- [ ] Verify VS Code extension updates in the marketplace
- [ ] Verify https://docs.rs/sanctifier-cli shows the new version
- [ ] Announce the release in relevant channels

## Rollback (if needed)

- [ ] Yank the crate: `cargo yank --vers X.Y.Z sanctifier-cli`
- [ ] Delete the GitHub release and re-tag
