# Artifact Provenance Verification Guide

Sanctifier provides cryptographically verifiable provenance for its vulnerability database and schema artifacts to ensure they have not been tampered with after being produced by our official CI pipeline.

For the threat model behind these controls — what they defend against and why each one exists — see [release-artifacts-threat-model.md](release-artifacts-threat-model.md).

## What is covered

The canonical release-artifact set is declared in [`data/release-manifest.json`](../data/release-manifest.json).
Every file listed there is hashed in `CHECKSUMS.txt` and attested via GitHub Artifact Attestations on each tag push.

## Verification using Checksums

Every release includes a `CHECKSUMS.txt` manifest. You can verify your local files using `sha256sum` (or `shasum -a 256` on macOS):

```bash
# Verify all files in the manifest
grep -v '^#' CHECKSUMS.txt | sha256sum -c
```

## Verification using GitHub Attestations

Sanctifier uses [GitHub Artifact Attestations](https://github.blog/2024-05-02-introducing-github-artifact-attestations-now-in-public-beta/) to provide SLSA-aligned provenance.

### Prerequisites

Install the [GitHub CLI](https://cli.github.com/):

```bash
brew install gh
```

### Verifying an Artifact

To verify the integrity and origin of an artifact (e.g., `vulnerability-db.json`), run:

```bash
gh attestation verify data/vulnerability-db.json --repo HyperSafeD/Sanctifier
```

This command confirms that:
1. The artifact was built by the official Sanctifier repository.
2. The specific build workflow and commit are recorded.
3. The artifact has not been modified since it was attested.

## Reporting Issues

If you find a mismatch or a failed attestation for an official release, please report it immediately via a [Security Advisory](https://github.com/HyperSafeD/Sanctifier/security/advisories/new) or by opening a critical issue.
