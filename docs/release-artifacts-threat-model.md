# Release Artifacts: Security Hardening + Threat Model

This document covers the supply-chain threat model for the JSON / YAML
artifacts that ship as part of every Sanctifier release — the contents
of `data/` and `schemas/` that downstream consumers (CI integrations,
auditors, formal-verification pipelines) rely on as a trusted source of
truth.

It complements [docs/provenance-verification.md](provenance-verification.md)
(which documents *how* to verify) with *why* each control exists.

## Scope

The release artifact set is canonically declared in
[`data/release-manifest.json`](../data/release-manifest.json).  Every
file listed there:

- is hashed in `CHECKSUMS.txt` (SHA-256, manifest-driven);
- is attested via [GitHub Artifact Attestations](https://github.blog/2024-05-02-introducing-github-artifact-attestations-now-in-public-beta/) on every tag push;
- is enforced by the lint pipeline (`make lint` / CI) to be parseable
  and prettier-formatted.

Anything *not* listed in the manifest is out-of-scope for the release
guarantees.  Adding a file to the manifest is a public commitment.

## Assets

| Asset | Why it matters |
|-------|----------------|
| `data/vulnerability-db.json` | Drives the regex-based vulnerability detector. A malicious entry could mask a real bug or fire false positives. |
| `data/sarif/severity-map.yaml` | Maps internal categories to SARIF severity. Demoting `critical` → `low` would silence real findings. |
| `data/sarif/rule-metadata.yaml` | The public rule catalogue. Tampering would mislead downstream tools that expect stable IDs. |
| `data/security-review/*.yaml` | Encodes the project's security-review policy. Tampering could disable the review-required gate. |
| `schemas/*.json` | The contract between Sanctifier and downstream consumers. Loosening `additionalProperties` or removing a `required` field is a silent compatibility break. |

## Threat model

The threats below are ordered roughly by likelihood × impact.

### T1 — Tampering after release

**Threat.** An attacker swaps a released artifact in transit, on a
mirror, or in a compromised package registry.

**Mitigations.**
- `CHECKSUMS.txt` is signed via `actions/attest-build-provenance` on
  every tag push.
- Consumers can verify with `gh attestation verify <file> --repo HyperSafeD/Sanctifier`.
- `scripts/generate-provenance.sh` writes the manifest atomically
  (temp file + `mv`), so a partial CHECKSUMS.txt cannot leak on
  interrupt.

**Residual risk.** A consumer that does not run `gh attestation verify`
trusts only the checksum file fetched from the same source as the
artifact.  Document the verification command prominently (see
[provenance-verification.md](provenance-verification.md)).

### T2 — Schema poisoning via PR

**Threat.** A pull request subtly weakens a schema (drops
`additionalProperties: false`, removes a `required` field, broadens an
`enum`) so a malicious payload validates against it.

**Mitigations.**
- `npm run lint:release-artifacts` (wired into `make lint` and CI)
  enforces:
  - `$schema = http://json-schema.org/draft-07/schema#` on every schema;
  - `schemas/analysis-output.json` retains a required `schema_version`
    field whose first example is strict semver (catches accidental
    version drift);
  - `data/vulnerability-db.json` declares strict semver `version` and
    ISO-8601 `last_updated`.
- `scripts/validate_db.js` validates every data file against its
  schema with AJV (catches inputs that no longer satisfy the contract
  after a schema change).

**Residual risk.** Reviewers must still inspect schema diffs.  Adding
a brand-new optional field is *intentional*; the lint cannot tell
intent from accident.

### T3 — Coverage drift

**Threat.** A new schema or data file ships unattested because the
release pipeline still hardcodes the old list.

**Mitigations.**
- `data/release-manifest.json` is the single source of truth.
- `scripts/verify-artifacts.sh` and `scripts/generate-provenance.sh`
  read the file list from the manifest — there are no hardcoded paths.
- `scripts/validate_release_artifacts.js` cross-checks
  `CHECKSUMS.txt` against the manifest and fails CI on either-direction
  drift (artifact in manifest but missing from CHECKSUMS, or in
  CHECKSUMS but missing from manifest).

### T4 — Schema downgrade / version pinning

**Threat.** A consumer accidentally pins to a stale schema version
that no longer reflects the live data file shape.

**Mitigations.**
- `schemas/analysis-output.json` carries an explicit `schema_version`
  field (currently `1.0.0`) — bump rules:
  - **Patch** (`1.0.x`): documentation tweaks, examples, descriptions.
  - **Minor** (`1.x.0`): additive fields, optional properties.
  - **Major** (`x.0.0`): removal, renaming, type changes, or
    `additionalProperties` tightening.  Major bumps require a
    migration note in [CHANGELOG.md](../CHANGELOG.md) under the
    matching `### Changed` / `### Removed` heading.
- The lint pipeline rejects a `schema_version` `examples[0]` that is
  not strict semver.

### T5 — Reproducibility failure

**Threat.** The same input produces different `CHECKSUMS.txt` bytes on
two runners, weakening the attestation guarantee.

**Mitigations.**
- `scripts/generate-provenance.sh` runs `./scripts/verify-artifacts.sh`
  as a precondition (parseability check; canonical *formatting* is
  owned by `prettier` to avoid a formatter-on-formatter conflict).
- The script prefers `sha256sum` and falls back to `shasum -a 256`
  with a deterministic invocation order.
- File order in the manifest is preserved verbatim — sorting happens
  once, at manifest authoring time, not per-run.

### T6 — Build environment compromise

**Out of scope** for this document; covered by the broader
[security-threat-model.md](security-threat-model.md).  Briefly:
release jobs run on `ubuntu-latest` with `permissions:
{contents: write, id-token: write, attestations: write}`, no other
write scopes.

## Operational guarantees

- **Output format stability.** `CHECKSUMS.txt` retains its line shape
  (`<hex>  <path>`) across this release; existing consumer scripts
  using `grep -v '^#' CHECKSUMS.txt | sha256sum -c` keep working.
- **Backwards compatibility.** Files attested in earlier releases
  (`data/vulnerability-db.json`, `schemas/analysis-output.json`,
  `schemas/sanctifier.json`) remain attested.  This release *expands*
  coverage; it does not contract or rename anything.
- **Migration notes.** None required for this release.  Future
  removals from `data/release-manifest.json` MUST come with a major
  version bump and a CHANGELOG entry under `### Removed`.

## How to add a new release artifact

1. Add the file to `data/release-manifest.json` under
   `artifacts.data` or `artifacts.schemas`, alphabetically sorted.
2. Run `scripts/generate-provenance.sh` to regenerate `CHECKSUMS.txt`.
3. Run `make lint` locally — the validator confirms manifest /
   checksum coherence before you push.
4. Note the addition in [CHANGELOG.md](../CHANGELOG.md) under
   `### Added`.

## How to remove a release artifact

1. Open a follow-up issue describing why the artifact is being dropped
   and what replaces it (if anything).
2. Bump the major version of any schema whose contract changes as a
   result.
3. Remove the entry from `data/release-manifest.json` and regenerate
   `CHECKSUMS.txt`.
4. Add a `### Removed` entry to [CHANGELOG.md](../CHANGELOG.md) with a
   migration note for downstream consumers.
