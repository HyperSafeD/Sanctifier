# GitHub Action Threat Model Notes

This document captures a lightweight threat model for the composite GitHub Action (`action.yml`) to help contributors and users reason about security boundaries and permissions.

## Assets

- **Repository contents**: source code and configuration being scanned.
- **`GITHUB_TOKEN`**: ephemeral token used by workflows; scope depends on workflow `permissions`.
- **Code scanning results** (SARIF): uploaded to GitHub when enabled.

## Trust boundaries

- **Action code**: runs on GitHub-hosted runners, executes shell commands, and installs the CLI via Cargo.
- **Network**: the action fetches dependencies when installing the CLI (Cargo registry and git dependencies).
- **Fork pull requests**: are treated as untrusted by default; SARIF upload is skipped for fork PRs.

## Threats & mitigations

### Excessive token permissions

**Threat**: broad workflow permissions increase blast radius if a dependency or step is compromised.

**Mitigation**: use the principle of least privilege in workflow `permissions`:

- `contents: read` for scanning
- add `security-events: write` **only** when SARIF upload is enabled

### Supply chain risk (CLI installation)

**Threat**: installing via `cargo install` downloads dependencies at runtime.

**Mitigations**:

- Prefer pinning a released action version tag (e.g. `HyperSafeD/Sanctifier@vX.Y.Z`) and, optionally, `with: version: X.Y.Z`.
- Keep action inputs constrained to repository-relative paths and documented output formats before invoking shell commands.
- Keep SARIF upload disabled on untrusted events (fork PRs are already handled).

### Data exfiltration via logs/artifacts

**Threat**: secrets or sensitive data could be printed to logs or written into artifacts/SARIF.

**Mitigations**:

- Do not pass secrets to the action inputs.
- Keep CI logs reviewed and avoid printing environment variables.
- Only upload SARIF when you intend to publish findings to the repository’s code scanning UI.

## Non-goals

- This document does not attempt to fully model GitHub-hosted runner isolation or GitHub’s internal security posture.
- This document does not guarantee vulnerability absence; it documents expected boundaries and safe defaults.
