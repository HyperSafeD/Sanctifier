# Contributing to Sanctifier

Welcome and thanks for contributing!

## Community Health Files

Before opening an issue or pull request, review the project community policies:

- [Code of Conduct](.github/CODE_OF_CONDUCT.md)
- [Bug Report Template](.github/ISSUE_TEMPLATE/bug_report.yml)
- [Feature Request Template](.github/ISSUE_TEMPLATE/feature_request.yml)
- [Pull Request Template](.github/PULL_REQUEST_TEMPLATE.md)
- [Security Policy](.github/SECURITY.md)

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

## Commit Message Convention

This project follows [Conventional Commits](https://www.conventionalcommits.org/) specification. All commit messages should be structured as follows:

```
<type>: <description>

[optional body]

[optional footer(s)]
```

### Commit Types

- `feat:` - A new feature
- `fix:` - A bug fix
- `perf:` - A code change that improves performance
- `test:` - Adding missing tests or correcting existing tests
- `docs:` - Documentation only changes
- `ci:` - Changes to CI configuration files and scripts
- `refactor:` - A code change that neither fixes a bug nor adds a feature (no behaviour change)
- `style:` - Changes that do not affect the meaning of the code (white-space, formatting, etc)
- `build:` - Changes that affect the build system or external dependencies
- `chore:` - Other changes that don't modify src or test files

### Examples

```
feat: add reentrancy detection for cross-contract calls

fix: correct overflow check in token transfer

perf: optimize WASM parsing for large contracts

docs: update deployment guide with Stellar testnet instructions

ci: add commitlint validation to PR workflow

refactor: extract common validation logic into helper module

test: add property-based tests for AMM pool
```

### Breaking Changes

Breaking changes should be indicated by a `!` after the type or by adding `BREAKING CHANGE:` in the footer:

```
feat!: change API response format for analysis results

BREAKING CHANGE: The analysis API now returns findings in a nested structure
```

## PR Process

- Create an issue or confirm there is already one.
- Fork the repository and create a branch: `git checkout -b issue-###-description`.
- Implement the code and run tests locally:
  - `cargo fmt --all`
  - `cargo test -p sanctifier-core --all-features`
  - `cargo test -p sanctifier-cli --no-default-features`
- Write commit messages following the Conventional Commits specification above.
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

## Supply-Chain Security

Sanctifier ensures the integrity of its vulnerability database and JSON schemas:

- **Deterministic Formatting**: All JSON artifacts in `data/` and `schemas/` must be pretty-printed. Run `./scripts/verify-artifacts.sh` to fix formatting.
- **Provenance Manifest**: A `CHECKSUMS.txt` file tracks SHA-256 hashes of critical artifacts.
- **Artifact Attestations**: Official releases include GitHub Artifact Attestations (SLSA-aligned) to prevent tampering.

Contributors should ensure that any changes to `data/` or `schemas/` are correctly formatted and that `CHECKSUMS.txt` is updated if required.

## QA checklist

- [ ] Branch created for specific issue
- [ ] CI passes on opened PR
- [ ] Peer review completed
- [ ] No direct push to main


---

## Code Style

### Rust

- Follow the standard `rustfmt` formatting (`cargo fmt --all`).
- Lint with `cargo clippy --all-targets --all-features -- -D warnings`.
- Use `snake_case` for functions and variables, `PascalCase` for types and traits.
- Prefer `Result<T, E>` over `panic!` / `unwrap()` in library code.
- Every public item must have a doc comment (`///`).
- Keep functions short and focused; extract helpers rather than nesting deeply.

### TypeScript / JavaScript

- Format with Prettier (`pnpm format`).
- Lint with ESLint (`pnpm lint`).
- Use `camelCase` for variables and functions, `PascalCase` for components and types.
- Prefer `const` over `let`; avoid `var`.
- All React components must be typed with explicit prop interfaces.
- No `any` types without a comment explaining why.

---

## Review SLA

| Stage | Target |
|---|---|
| First response (triage / acknowledgement) | **3 business days** |
| Full review (approve / request changes) | **5 business days** |
| Re-review after changes | **2 business days** |

If your PR has not received a response within the SLA, ping `@HyperSafeD` in the PR thread.

---

## Label Glossary

| Label | Meaning |
|---|---|
| `type: bug` | Something is broken or behaves incorrectly |
| `type: feature` | New capability or enhancement |
| `type: docs` | Documentation-only change |
| `type: refactor` | Code restructuring with no behaviour change |
| `type: test` | Test additions or fixes |
| `area: core-engine` | Changes to `tooling/sanctifier-core` |
| `area: frontend` | Changes to `frontend/` |
| `area: contracts` | Changes to `contracts/` |
| `area: docs` | Changes to documentation files |
| `area: testing` | Test infrastructure or coverage |
| `difficulty: easy` | Good for first-time contributors; well-scoped |
| `difficulty: medium` | Requires familiarity with the codebase |
| `difficulty: hard` | Complex; discuss approach before starting |
| `priority: high` | Blocking or time-sensitive |
| `priority: medium` | Important but not blocking |
| `priority: low` | Nice-to-have |
| `good first issue` | Recommended starting point for new contributors |
| `Stellar Wave` | Part of the Stellar Wave contributor programme |
| `status: blocked` | Waiting on another issue or external dependency |
| `status: needs-info` | Awaiting clarification from the reporter |
| `status: wip` | Work in progress — do not pick up |
