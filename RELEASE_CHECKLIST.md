# Release Checklist

This document describes the steps required to publish a new release of Sanctifier.

## Pre-release

- [ ] Ensure all CI checks pass on `main` (build, test, lint, coverage)
- [ ] Run `scripts/release.sh X.Y.Z` to bump all version strings
  - `Cargo.toml` (workspace)
  - `tooling/sanctifier-cli/Cargo.toml` (including sanctifier-core dep)
  - `tooling/sanctifier-core/Cargo.toml`
  - `tooling/sanctifier-detector/Cargo.toml`
  - `tooling/sanctifier-wasm/Cargo.toml`
  - `vscode-extension/package.json`
  - `packages/sanctifier-cli-npm/package.json`
  - `homebrew/sanctifier.rb`
- [ ] Update `CHANGELOG.md` — move Unreleased entries to a new `[X.Y.Z]` section
- [ ] Run `cargo publish --dry-run -p sanctifier-core && cargo publish --dry-run -p sanctifier-cli` and fix any warnings
- [ ] Run `cargo publish --dry-run -p sanctifier-detector` and fix any warnings
- [ ] Verify all documentation links in `README.md` and `docs/` are valid
- [ ] Check that `CARGO_REGISTRY_TOKEN` is set in GitHub Secrets
- [ ] Check that `NPM_TOKEN` is set in GitHub Secrets (for @hypersafed/sanctifier-cli)

## Release

- [ ] Commit version bump: `git commit -am "chore: bump version to X.Y.Z"`
- [ ] Create and push an annotated tag: `git tag -a vX.Y.Z -m "Release vX.Y.Z" && git push origin main && git push origin vX.Y.Z`
- [ ] Wait for the `Release` workflow to build and attach binaries
- [ ] Wait for the `Publish to Crates.io` workflow to complete
- [ ] Wait for the `Publish to npm` workflow to publish @hypersafed/sanctifier-cli
- [ ] Wait for the `Docker` workflow to push to ghcr.io/hypersafed/sanctifier
- [ ] Wait for the `Homebrew` workflow to update HyperSafeD/homebrew-sanctifier
- [ ] Wait for the `Publish API Documentation` workflow to deploy docs
- [ ] Wait for the `Release VS Code Extension` workflow to publish to VS Code Marketplace

## Post-release

- [ ] Verify `cargo install sanctifier-cli` installs the latest version
- [ ] Verify `npx @hypersafed/sanctifier-cli analyze ./my-contract` works
- [ ] Verify `docker run --rm -v $(pwd):/workspace ghcr.io/hypersafed/sanctifier analyze /workspace` works
- [ ] Verify `brew install HyperSafeD/sanctifier/sanctifier` works
- [ ] Verify the GitHub release page shows correct binaries and SHA256SUMS
- [ ] Verify https://crates.io/crates/sanctifier-cli shows the new version
- [ ] Verify https://www.npmjs.com/package/@hypersafed/sanctifier-cli shows the new version
- [ ] Verify https://github.com/HyperSafeD/packages shows the Docker image
- [ ] Verify `winget install HyperSafeD.Sanctifier` works
- [ ] Verify `scoop install sanctifier` works
- [ ] Verify VS Code extension updates in the marketplace
- [ ] Verify https://docs.rs/sanctifier-cli shows the new version
- [ ] Announce the release in relevant channels

## Rollback (if needed)

- [ ] Yank the crate: `cargo yank --vers X.Y.Z sanctifier-cli`
- [ ] Unpublish the npm package: `npm unpublish @hypersafed/sanctifier-cli@X.Y.Z`
- [ ] Delete the GitHub release and re-tag
