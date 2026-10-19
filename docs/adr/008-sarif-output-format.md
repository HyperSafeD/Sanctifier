# ADR 008: Use SARIF as the Primary Output Format

## Status

Accepted

## Context

Sanctifier produces analysis findings that need to be consumed by multiple downstream
surfaces: the CLI (human-readable terminal output), CI pipelines (machine-readable for
pass/fail gating), IDE extensions (VS Code Problems panel), GitHub Advanced Security
(code-scanning alerts), and the web dashboard.

Each surface has different needs. We need a wire format that:

1. Carries enough metadata (rule id, severity, location with line/column, help text) to
   power all downstream surfaces without lossy re-serialization.
2. Is consumable by existing tooling without custom parsers.
3. Supports GitHub code-scanning upload natively.

Formats evaluated:

| Format | Ecosystem fit | GitHub GHAS | IDE support | Structured location |
|--------|--------------|-------------|-------------|---------------------|
| **SARIF 2.1.0** | Security tooling standard (OASIS) | Native | VS Code SARIF Viewer | Yes (region object) |
| Plain JSON | Universal | No (needs transformation) | No native viewer | Custom |
| CSV | Simple tooling | No | No | Column-based |
| Checkstyle XML | Java-focused CI | No | Some | Line only |
| ESLint JSON | JS-focused | No | No | Limited |

## Decision

We use **SARIF 2.1.0** (Static Analysis Results Interchange Format, OASIS standard) as
the canonical interchange format for Sanctifier findings.

Every internal `Finding` is serialized to a SARIF `result` object inside a `run` with
a `tool.driver` that lists all rules with their ids, names, and help URIs. The CLI
renders this to human-friendly text; every other surface consumes the SARIF JSON
directly.

SARIF is produced by `sanctifier analyze --format sarif` and is the format uploaded to
GitHub code-scanning via `actions/upload-sarif`.

## Alternatives Considered

### Plain JSON (custom schema)

A custom JSON schema would be simpler to implement initially but would require every
consumer (VS Code extension, dashboard, CI script) to implement its own parser. SARIF
already has parsers in most languages and is supported natively by GitHub and VS Code.

### Checkstyle XML

Checkstyle is widely understood by Java CI systems and Jenkins plugins. It is not
supported natively by GitHub code-scanning and carries no rule-metadata or help-text
fields beyond line/column. Insufficient for our needs.

### ESLint JSON format

ESLint's JSON output is familiar to front-end developers but is JS-ecosystem-specific
and carries no formal schema. It has no native GitHub code-scanning adapter.

## Consequences

**Positive:**
- GitHub code-scanning ingests SARIF natively — findings appear as inline PR annotations
  with zero additional tooling.
- The VS Code SARIF Viewer extension renders `sanctifier.sarif` files without any
  Sanctifier-specific plugin code.
- SARIF's `rule` objects carry `helpUri`, `shortDescription`, and `fullDescription`,
  enabling rich hover documentation in IDEs.
- The format is versioned and schema-validated; breaking changes require an OASIS
  standard update.

**Negative:**
- SARIF JSON is verbose; a report with 50 findings is 10–20× larger than a minimal
  custom JSON schema would be.
- The SARIF spec is large (200+ pages). Contributors need to understand region offsets,
  artifact locations, and run-level properties to extend the serializer correctly.
- SARIF does not have first-class support for "fix" objects in 2.1.0 (fixes are a
  proposed extension). Quick-fix code actions in the VS Code extension are implemented
  separately in the extension layer.
