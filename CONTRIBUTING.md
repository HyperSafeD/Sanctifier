# Contributing to Sanctifier

Welcome and thanks for contributing!

## Quick Start with GitHub Codespaces

The fastest way to start contributing is using GitHub Codespaces, which provides a pre-configured development environment with all dependencies installed:

1. Click the "Code" button on the repository page
2. Select the "Codespaces" tab
3. Click "Create codespace on main" (or your branch)

The devcontainer will automatically install:

- Rust toolchain
- Z3 theorem prover
- soroban-cli
- wasm-pack
- VS Code extensions (rust-analyzer, even-better-toml)

After the container builds, all dependencies will be ready and `cargo build --workspace` will have completed.

## Local Development Setup

If you prefer to develop locally, you'll need to install:

- Rust 1.78+
- Z3 (`libz3-dev` on Debian/Ubuntu, `z3` via Homebrew on macOS)
- Clang/LLVM (`clang` and `libclang-dev` on Debian/Ubuntu, `llvm` via Homebrew on macOS)
- soroban-cli: `cargo install soroban-cli`
- wasm-pack: `cargo install wasm-pack`

## PR Process

- Create an issue or confirm there is already one.
- Fork the repository and create a branch: `git checkout -b issue-###-description`.
- Implement the code and run tests locally:
  - `cargo fmt --all`
  - `cargo test -p sanctifier-core --all-features`
  - `cargo test -p sanctifier-cli --no-default-features`
- Push to your fork and open a PR to `HyperSafeD/Sanctifier:main`.
- Ensure that the PR is checked by CI and that all required status checks pass.
- Seek at least one approving review.

## Branch Protection

This repo uses branch protection for `main`:

- Required status check: `Continuous Integration`
- Require branches to be up to date before merging
- Require at least 1 review approval
- Disallow force pushes

See `BRANCH_PROTECTION.md` for details.

## Code Style

- Use `cargo fmt --all` for formatting.
- Use `cargo clippy` for lint checks.

## QA checklist

- [ ] Branch created for specific issue
- [ ] CI passes on opened PR
- [ ] Peer review completed
- [ ] No direct push to main
