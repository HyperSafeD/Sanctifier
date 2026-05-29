# Troubleshooting Guide

This guide is the module boundary map for troubleshooting content across docs/specs. Keep detailed fixes near the system that owns them, and use this page as the stable entry point from `DOCUMENTATION_INDEX.md`.

## Module boundaries

| Problem area                        | Canonical owner                                                           | Put here                                                                                             |
| ----------------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Stellar/Soroban deployment failures | `docs/soroban-deployment.md`                                              | Secret, account funding, RPC, contract invocation, deployment validation, and network health fixes.  |
| CI/CD or GitHub Actions failures    | `docs/ci-cd-setup.md`                                                     | Workflow triggers, repository secrets, artifact retention, branch protection, and CI rerun guidance. |
| Fast local diagnosis                | `docs/QUICK_REFERENCE.md`                                                 | Short commands for checking tools, logs, deployment manifests, and validation output.                |
| Runtime guard integration           | `docs/runtime-guards-integration.md`                                      | Contract integration issues involving guard wrappers and invariant checks.                           |
| Specification drift                 | `specs/sep41_token_total_supply.tla` and `docs/docs-specs-ci-coverage.md` | SEP-41 invariant ownership and docs/specs CI expectations.                                           |

## Triage flow

1. Start with `docs/QUICK_REFERENCE.md` when the symptom is local and command-oriented.
2. Move to `docs/soroban-deployment.md` when the failure involves Stellar accounts, Soroban RPC, deployment, or validation behavior.
3. Move to `docs/ci-cd-setup.md` when the failure only appears in GitHub Actions or repository automation.
4. Run `npm run docs:specs:check` after editing troubleshooting links or section names.

## Contributor checklist

- Keep one canonical fix per problem type; cross-link instead of duplicating long command blocks.
- Keep output examples stable unless the underlying command format changed.
- Add or update docs/specs validation when a new troubleshooting owner page is introduced.
- Link new troubleshooting pages from `DOCUMENTATION_INDEX.md`.
