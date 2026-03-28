# Branch Protection Policy

This repository follows strict branch protection rules for `main` to ensure code quality and stability.

## Required protections (GitHub Settings > Branches > main)

1. Require status checks to pass before merging
   - Required checks:
     - `Continuous Integration` (from `.github/workflows/ci.yml`)
2. Require branches to be up to date before merging
3. Require at least 1 code review approval
4. Disable force pushes to `main`

## For fork workflows

1. Fork the repository.
2. Create a feature branch in your fork (e.g. `issue-310-branch-protection`).
3. Implement and test locally.
4. Open a PR from your fork branch to `HyperSafeD/Sanctifier:main`.
5. Ensure CI passes and at least 1 review approves before merge.

## Required status checks

- `Continuous Integration` (as configured in `.github/workflows/ci.yml`)
  - Linux, macOS, Windows matrix
  - Format check, Clippy, build, tests, coverage

## Why this exists

Branch protection mitigates accidental merges of failing or unreviewed code and ensures that `main` is always in deployable condition.