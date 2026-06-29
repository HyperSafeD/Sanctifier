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

### Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** 1.78+ ([rustup](https://rustup.rs/))
- **Node.js** 24+ ([nvm](https://github.com/nvm-sh/nvm) recommended)
- **Z3 Theorem Prover**:
  - Debian/Ubuntu: `sudo apt-get install -y libz3-dev`
  - macOS: `brew install z3 llvm`
  - Windows: Download from [Z3Prover/z3 releases](https://github.com/Z3Prover/z3/releases)
- **Clang/LLVM** (for Z3 bindings):
  - Debian/Ubuntu: `sudo apt-get install -y clang libclang-dev`
  - macOS: `brew install llvm`
- **soroban-cli**: `cargo install soroban-cli`
- **wasm-pack**: `cargo install wasm-pack`

### One-Command Setup

Run the automated setup script to install all prerequisites:

```bash
make dev-setup
```

This will:
- Update and set the stable Rust toolchain
- Add the WASM target
- Install wasm-pack and soroban-cli
- Install Node.js dependencies

After running `make dev-setup`, you'll need to manually install Z3 (see platform-specific instructions above).

### Manual Setup Steps

If you prefer to set up manually:

1. **Install Rust toolchain**:
   ```bash
   rustup update stable
   rustup default stable
   rustup target add wasm32-unknown-unknown
   ```

2. **Install wasm-pack**:
   ```bash
   cargo install wasm-pack
   ```

3. **Install soroban-cli**:
   ```bash
   cargo install soroban-cli
   ```

4. **Install Node.js dependencies**:
   ```bash
   npm install
   ```

5. **Install Z3** (platform-specific, see Prerequisites above)

6. **Build the project**:
   ```bash
   make build
   ```

7. **Run tests**:
   ```bash
   make test
   ```

## Architecture Overview

Sanctifier follows a 5-layer architecture for analyzing Soroban smart contracts:

```
┌─────────────────────────────────────────────────────────────┐
│                         USER INTERFACE                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │     CLI      │  │  Web UI      │  │  GitHub Action   │  │
│  │  (sanctifier │  │  (Frontend)  │  │  (Automation)    │  │
│  │   deploy)    │  │              │  │                  │  │
│  └──────┬───────┘  └──────┬───────┘  └───────┬──────────┘  │
└─────────┼─────────────────┼─────────────────┼──────────────┘
          │                 │                 │
          └─────────────────┼─────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                      ORCHESTRATION LAYER                     │
│  • CLI command parsing & validation                          │
│  • GitHub Actions workflow coordination                      │
│  • Configuration management                                  │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                        ANALYSIS ENGINE                        │
│  • Rule execution & scheduling                               │
│  • Finding aggregation & deduplication                       │
│  • Severity scoring & filtering                              │
│  • Report generation (SARIF, JSON)                           │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                         RULES LAYER                            │
│  • 50+ security rules (reentrancy, overflow, access control) │
│  • Pattern matching & AST traversal                          │
│  • Z3-based formal verification (optional)                   │
│  • Custom rule support via YAML config                       │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                          PARSER LAYER                          │
│  • Soroban contract source parsing                           │
│  • Rust syntax tree (syn) extraction                         │
│  • WASM artifact analysis                                    │
│  • Contract interface discovery                              │
└──────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

1. **Parser Layer** (`tooling/sanctifier-core/src/parser/`)
   - Parses Soroban smart contract source code
   - Extracts Rust syntax trees using `syn`
   - Analyzes WASM artifacts when available
   - Discovers contract interfaces and entry points

2. **Rules Layer** (`tooling/sanctifier-core/src/rules/`)
   - Implements 50+ security analysis rules
   - Pattern matching for common vulnerabilities (reentrancy, overflow, etc.)
   - Z3-based formal verification for invariant checking
   - Custom rule support via YAML configuration

3. **Engine Layer** (`tooling/sanctifier-core/src/engine/`)
   - Orchestrates rule execution
   - Aggregates and deduplicates findings
   - Applies severity scoring and filtering
   - Generates reports in SARIF, JSON, and other formats

4. **Orchestration Layer** (`tooling/sanctifier-cli/`)
   - CLI command parsing and validation
   - GitHub Actions workflow coordination
   - Configuration and environment management
   - Deployment automation for runtime guard wrappers

5. **User Interface Layer** (`frontend/`, `tooling/sanctifier-cli/`)
   - Web-based UI for interactive analysis
   - CLI for command-line usage
   - GitHub Action for CI/CD integration
   - Results visualization and reporting

## How to Add a New Rule

Follow these steps to contribute a new security analysis rule:

### 1. Understand the Rule Requirements

- Identify the vulnerability pattern you want to detect
- Review existing rules in `tooling/sanctifier-core/src/rules/` for similar patterns
- Determine if the rule requires Z3 formal verification or can use pattern matching
- Define the rule's severity (Low, Medium, High, Critical)

### 2. Create the Rule Implementation

Create a new file in `tooling/sanctifier-core/src/rules/` (e.g., `your_rule_name.rs`):

```rust
use sanctifier_core::rules::{Rule, RuleContext, Finding, Severity};

pub struct YourRuleName;

impl Rule for YourRuleName {
    fn id(&self) -> &'static str {
        "YOUR-RULE-ID"  // e.g., "R001"
    }
    
    fn name(&self) -> &'static str {
        "Your Rule Name"
    }
    
    fn description(&self) -> &'static str {
        "Description of what this rule detects"
    }
    
    fn severity(&self) -> Severity {
        Severity::High  // or Low, Medium, Critical
    }
    
    fn check(&self, context: &RuleContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        
        // Your analysis logic here
        // Use context.ast, context.contract_info, etc.
        
        findings
    }
}
```

### 3. Register the Rule

Add your rule to the rule registry in `tooling/sanctifier-core/src/rules/mod.rs`:

```rust
mod your_rule_name;

pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        // ... existing rules
        Box::new(your_rule_name::YourRuleName),
    ]
}
```

### 4. Write Tests

Create comprehensive tests in `tooling/sanctifier-core/tests/`:

- **Positive tests**: Contracts that should trigger the rule
- **Negative tests**: Contracts that should NOT trigger the rule
- **Edge cases**: Boundary conditions and unusual patterns

Example test structure:

```rust
#[test]
fn test_your_rule_detects_vulnerability() {
    let contract = r#"
        // Vulnerable contract code
    "#;
    
    let findings = analyze_contract(contract);
    assert!(findings.iter().any(|f| f.rule_id == "YOUR-RULE-ID"));
}
```

### 5. Update Documentation

- Add your rule to `docs/rules/` with:
  - Rule ID and name
  - Description of what it detects
  - Example vulnerable code
  - Example fixed code
  - Severity and category

### 6. Run Tests

Ensure all tests pass:

```bash
# Run core tests
cargo test -p sanctifier-core --all-features

# Run your specific rule tests
cargo test -p sanctifier-core test_your_rule

# Run linting
cargo fmt --all
cargo clippy -p sanctifier-core -- -D warnings
```

### 7. Submit a Pull Request

Follow the PR process outlined below.

## PR Checklist

Before submitting a pull request, ensure the following:

### Code Quality

- [ ] Code follows Rust style guidelines (`cargo fmt --all`)
- [ ] No Clippy warnings (`cargo clippy --workspace -- -D warnings`)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] New code has appropriate test coverage (>80%)
- [ ] Public APIs have documentation comments (`///`)
- [ ] No unwrap() in library code (use `?` or proper error handling)

### Testing

- [ ] Unit tests added for new functionality
- [ ] Integration tests pass
- [ ] Frontend tests pass (if applicable): `cd frontend && npm test`
- [ ] E2E tests pass (if applicable): `cd frontend && npm run test:e2e`

### Documentation

- [ ] `CONTRIBUTING.md` updated (if adding new workflows/processes)
- [ ] `ARCHITECTURE.md` updated (if changing system architecture)
- [ ] Rule documentation added in `docs/rules/` (if adding analysis rules)
- [ ] README.md updated (if adding user-facing features)
- [ ] CHANGELOG.md updated with your changes

### CI/CD

- [ ] GitHub Actions workflows pass
- [ ] No new warnings in CI logs
- [ ] Code coverage maintained or improved

### Commit Messages

- [ ] Follows Conventional Commits format: `type(scope): description`
- [ ] Types: `feat`, `fix`, `docs`, `test`, `refactor`, `ci`, etc.
- [ ] Descriptive subject line (50 chars or less)
- [ ] Body explains "what" and "why" (not "how")

Example:
```
feat(rules): add detection for integer overflow in token transfers

Implements rule R050 to detect potential integer overflow vulnerabilities
in SPL token transfer operations. Uses pattern matching on arithmetic
operations in transfer functions.

Closes #123
```

## Code Style Guide

### Rust

- **Formatting**: Use `cargo fmt --all` (standard rustfmt config)
- **Linting**: Use `cargo clippy --all-targets --all-features -- -D warnings`
- **Naming**:
  - `snake_case` for functions and variables
  - `PascalCase` for types and traits
  - `SCREAMING_SNAKE_CASE` for constants
- **Error Handling**: Prefer `Result<T, E>` over `panic!` / `unwrap()` in library code
- **Documentation**: Every public item must have a doc comment (`///`)
- **Function Size**: Keep functions short and focused; extract helpers rather than nesting deeply
- **Imports**: Group imports (std, external crates, internal modules) with blank lines between groups

### TypeScript / JavaScript (Frontend)

- **Formatting**: Use Prettier (`npm run format` or `pnpm format`)
- **Linting**: Use ESLint (`npm run lint` or `pnpm lint`)
- **Naming**:
  - `camelCase` for variables and functions
  - `PascalCase` for components and types
  - `UPPER_SNAKE_CASE` for constants
- **Type Safety**: 
  - All React components must be typed with explicit prop interfaces
  - No `any` types without a comment explaining why
  - Prefer `interface` over `type` for object shapes
- **Components**: 
  - One component per file
  - Co-locate styles with components
  - Use functional components with hooks

### YAML (GitHub Actions, Config)

- Use 2-space indentation
- Quote strings that contain special characters
- Use `${{ }}` syntax for GitHub Actions expressions
- Group related steps with comments

### Markdown (Documentation)

- Use ATX-style headers (`#` not `===` underlines)
- Wrap lines at 80 characters for readability
- Use fenced code blocks with language identifiers
- Include alt text for images

## Commit Message Convention

This project follows [Conventional Commits](https://www.conventionalcommits.org/) specification. All commit messages should be structured as follows:

```
<type>(<scope>): <description>

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
feat(rules): add reentrancy detection for cross-contract calls

fix(parser): correct overflow check in token transfer

perf(engine): optimize WASM parsing for large contracts

docs: update deployment guide with Stellar testnet instructions

ci: add commitlint validation to PR workflow

refactor(rules): extract common validation logic into helper module

test: add property-based tests for AMM pool
```

### Breaking Changes

Breaking changes should be indicated by a `!` after the type or by adding `BREAKING CHANGE:` in the footer:

```
feat(api)!: change analysis result format

BREAKING CHANGE: The analysis API now returns findings in a nested structure
```

## PR Process

1. **Create an issue** or confirm there is already one describing the problem/feature.
2. **Fork the repository** and create a branch: `git checkout -b issue-###-description`.
3. **Implement the code** following the guidelines above.
4. **Run tests locally**:
   ```bash
   make lint
   cargo test --workspace
   cd frontend && npm test
   ```
5. **Write commit messages** following the Conventional Commits specification.
6. **Push to your fork** and open a PR to `HyperSafeD/Sanctifier:main`.
7. **Ensure CI passes** - all required status checks must be green.
8. **Seek review** - request at least one approving review.
9. **Address feedback** - make requested changes and push updates.
10. **Merge** - once approved and CI passes, a maintainer will merge.

## Branch Protection

This repo uses branch protection for `main`:

- Required status check: `Continuous Integration`
- Require branches to be up to date before merging
- Require at least 1 review approval
- Disallow force pushes

See `BRANCH_PROTECTION.md` for details.

## Supply-Chain Security

Sanctifier ensures the integrity of its vulnerability database and JSON schemas:

- **Deterministic Formatting**: All JSON artifacts in `data/` and `schemas/` must be pretty-printed. Run `./scripts/verify-artifacts.sh` to fix formatting.
- **Provenance Manifest**: A `CHECKSUMS.txt` file tracks SHA-256 hashes of critical artifacts.
- **Artifact Attestations**: Official releases include GitHub Artifact Attestations (SLSA-aligned) to prevent tampering.

Contributors should ensure that any changes to `data/` or `schemas/` are correctly formatted and that `CHECKSUMS.txt` is updated if required.

## QA Checklist

Before submitting your PR, verify:

- [ ] Branch created for specific issue
- [ ] All tests pass locally (`make test`)
- [ ] Linting passes (`make lint`)
- [ ] Frontend tests pass (if applicable)
- [ ] Documentation updated
- [ ] Commit messages follow Conventional Commits
- [ ] CI passes on opened PR
- [ ] Peer review completed
- [ ] No direct push to main

## Review SLA

| Stage | Target |
|---|---|
| First response (triage / acknowledgement) | **3 business days** |
| Full review (approve / request changes) | **5 business days** |
| Re-review after changes | **2 business days** |

If your PR has not received a response within the SLA, ping `@HyperSafeD` in the PR thread.

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

---

## Getting Help

- **Documentation**: Check [docs/](docs/) for detailed guides
- **Issues**: Search [existing issues](https://github.com/HyperSafeD/Sanctifier/issues) or create a new one
- **Discussions**: Use [GitHub Discussions](https://github.com/HyperSafeD/Sanctifier/discussions) for questions
- **Security**: See [SECURITY.md](SECURITY.md) for vulnerability reporting

Thank you for contributing to Sanctifier! 🛡️