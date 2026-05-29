# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- CHANGELOG.md to track project changes
- Conventional Commits specification for commit messages
- `data/release-manifest.json` — single source of truth for release artifacts
  (data/ + schemas/) consumed by the provenance pipeline.
- `scripts/validate_release_artifacts.js` and the `npm run lint:release-artifacts`
  target — verifies schema `$schema` declarations, version stability of
  `analysis-output.json` `schema_version`, and CHECKSUMS coverage.
- `docs/release-artifacts-threat-model.md` — threat model and operational
  guarantees for release artifacts.

### Changed

- `scripts/verify-artifacts.sh` and `scripts/generate-provenance.sh` are now
  manifest-driven (read file list from `data/release-manifest.json`) and use
  `set -euo pipefail`, atomic temp-file writes, and a portable
  `sha256sum` / `shasum -a 256` fallback.
- `CHECKSUMS.txt` now covers every release artifact declared in the manifest
  (12 files, up from 3).  The line format is unchanged; existing
  `grep -v '^#' CHECKSUMS.txt | sha256sum -c` workflows keep working.
- `.github/workflows/provenance.yml` attests every manifest artifact plus
  `CHECKSUMS.txt` (subject list is computed from the manifest, not hardcoded).

### Deprecated

### Removed

### Fixed

### Security

- Closes the coverage gap where 3 of 6 schemas and 5 of 6 data files shipped
  unattested.  Tampering with a previously-uncovered file is now detectable
  via `gh attestation verify` or `sha256sum -c CHECKSUMS.txt`.
