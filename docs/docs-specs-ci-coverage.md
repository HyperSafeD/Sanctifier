# Docs and Specs CI Coverage

This note defines the docs/specs ownership and CI checks for Stellar ecosystem integration docs. It is intentionally small and executable: CI runs the same checks that contributors can run locally.

## Owner modules/files

| Area                              | Owner files                                                                                                            | Responsibility                                                                                                     |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Stellar/Soroban integration guide | `docs/soroban-deployment.md`                                                                                           | End-to-end deployment behavior, secrets, validation, and network operations.                                       |
| CI integration docs               | `docs/ci-cd-setup.md`                                                                                                  | GitHub Actions setup, Playwright/frontend coverage references, deployment artifacts, and workflow troubleshooting. |
| Troubleshooting entry point       | `docs/troubleshooting-guide.md`, `docs/QUICK_REFERENCE.md`                                                             | Boundaries for where fixes, commands, and escalation notes belong.                                                 |
| Support/compatibility matrix      | `docs/github-action-support-matrix.md`, `action.yml`, `scripts/action_inputs.py`, `tests/action/test_action_inputs.py` | Supported GitHub Action runners, inputs, input validation behavior, and GitHub annotation error messages.          |
| API reference generation          | `docs/api-reference-generation.md`, `Makefile`                                                                         | Rustdoc generation behavior and contribution notes for API docs.                                                   |
| Formal/spec coverage              | `specs/sep41_token_total_supply.tla`                                                                                   | SEP-41 total-supply invariant coverage for Stellar token behavior.                                                 |
| CI enforcement                    | `.github/workflows/ci.yml`, `scripts/validate_docs_specs.js`, `package.json`                                           | Integration checks that keep docs, specs, and index links aligned.                                                 |

## Integration/e2e coverage contract

CI runs a docs/specs integration check named `Docs/specs integration coverage`. The job installs Node dependencies and runs:

```bash
npm run docs:specs:check
```

The validator checks that:

- The documentation index links the canonical docs/specs maintenance pages.
- Stellar/Soroban owner files and the SEP-41 spec remain present.
- The support/compatibility matrix documents supported inputs and the action helper keeps explicit invalid-input errors.
- Troubleshooting, API reference generation, and docs/specs CI behavior keep their expected contributor-facing sections.
- The SEP-41 spec still exposes the total-supply invariant and core operation coverage.

This is an integration check for documentation behavior: it verifies that contributors can navigate from the index to the owner docs and that CI continues to cover the docs/specs contract.

## Local workflow

Before opening a PR that changes `docs/`, `specs/`, `.github/workflows/ci.yml`, `Makefile`, or docs-related scripts, run:

```bash
npm ci
npm run docs:specs:check
```

If the change updates Rust public APIs, also run:

```bash
make docs
```

## Stable output policy

This change does not alter Sanctifier CLI, SARIF, JSON, schema, or contract output formats. Docs/specs checks are additive. If a future docs/specs change requires a generated output format change, include a version bump and migration note in the nearest canonical format document before updating CI expectations.
