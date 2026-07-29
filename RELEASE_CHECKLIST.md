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

## Mainnet Release Addendum (#1140)

> **⚠️ This addendum is mandatory for any release tagged `v1.0.0-mainnet*`.**  
> It MUST be completed, filled out with named sign-offs, and attached as a comment to the release PR **before** the release tag is pushed.

This addendum layers an additional non-skippable sign-off gate on top of the general release checklist. Mainnet releases carry materially higher stakes and require explicit, named approval against every relevant mainnet-readiness criterion.

### Prerequisites

Before the sign-off can be completed, the following must all be satisfied:

- [ ] All issues in the [Mainnet Launch Readiness milestone](https://github.com/HyperSafeD/Sanctifier/milestones) are resolved or explicitly deferred with a documented rationale.
- [ ] The [`mainnet-fork-ci`](.github/workflows/mainnet-fork-ci.yml) workflow is passing on the release candidate branch.
- [ ] The release candidate branch has been frozen and is receiving only blocker fixes.

### Sign-Off Checklist

Each of the following items must be reviewed and signed off by **two named individuals** (the Release Manager and at least one independent Second Reviewer). Place an `X` in the checkbox and write your name & date below each item.

```
Example:
- [X] **Release Manager:** Alice Smith — 2026-07-28
- [X] **Second Reviewer:** Bob Chen — 2026-07-28
```

---

**1. Security checklist (`SECURITY.md` addendum) — #1115**

Verify that the mainnet security checklist published in `SECURITY.md` has been reviewed and all applicable items are satisfied.

- [ ] **Release Manager:**
- [ ] **Second Reviewer:**

---

**2. Formal-verification (Kani + Z3) coverage — #1114**

Confirm that a full formal-verification pass using Kani has been completed on every contract in scope. See [`docs/kani-integration.md`](docs/kani-integration.md) for the integration strategy and `contracts/kani-poc/` for proof-harness examples.

- [ ] **Release Manager:**
- [ ] **Second Reviewer:**

---

**3. Rollback / circuit-breaker procedure — #1137**

Confirm that a documented rollback and circuit-breaker procedure exists and has been reviewed for failed mainnet deployments. See [`ROLLBACK_PROCEDURE.md`](./ROLLBACK_PROCEDURE.md) for the full procedure.

- [ ] **Release Manager:**
- [ ] **Second Reviewer:**

---

**4. External security audit — #1112**

Confirm that an external smart-contract security audit has been completed and all critical/high findings are resolved or explicitly accepted with a risk rationale.

- [ ] **Release Manager:**
- [ ] **Second Reviewer:**

---

**5. Bug bounty program — #1116**

Confirm that the bug bounty program (Immunefi / HackenProof) is live and accepting submissions before the mainnet release tag is cut.

- [ ] **Release Manager:**
- [ ] **Second Reviewer:**

---

**6. Deployment safety — #1133, #1134, #1135**

Confirm that the `--confirm-mainnet` safety flag is enforced in deploy tooling, the end-to-end deployment runbook is complete, and a dry-run has passed.

- [ ] **Release Manager:**
- [ ] **Second Reviewer:

## Rollback (if needed)

- [ ] Yank the crate: `cargo yank --vers X.Y.Z sanctifier-cli`
- [ ] Unpublish the npm package: `npm unpublish @hypersafed/sanctifier-cli@X.Y.Z`
- [ ] Delete the GitHub release and re-tag

