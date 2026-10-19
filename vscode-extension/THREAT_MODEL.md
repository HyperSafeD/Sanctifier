# Sanctifier VS Code Extension — Threat Model

## Scope

This document covers the security threat model for the Sanctifier VS Code extension (`sanctifier-soroban`). It addresses the SARIF import/export feature added in issue #610.

## Assets

| Asset | Description |
|-------|-------------|
| Source files | Rust/Soroban files opened in the editor |
| SARIF exports | Analysis results written to disk |
| SARIF imports | External SARIF files read by the extension |
| Workspace trust | The VS Code workspace trust boundary |

## Trust Boundary

The extension runs inside the VS Code extension host process with the permissions of the logged-in OS user. It does **not** communicate over the network. All file I/O is local.

## Threat Analysis

### T1 — Malicious SARIF import (path traversal)

**Threat:** An attacker-crafted `.sarif` file with a `uri` containing `../` sequences could trick the extension into displaying paths outside the workspace.

**Mitigations:**
- `isPathWithinWorkspace()` in `security.ts` resolves both the workspace root and the file path with `path.resolve()` and checks the prefix before importing.
- The import dialog constrains file selection to the local filesystem.

### T2 — Oversized SARIF file (denial of service)

**Threat:** A very large SARIF file could exhaust memory or freeze the extension host when parsed.

**Mitigations:**
- `validateSarifContent()` enforces a 5 MB byte-length limit before parsing (`MAX_SARIF_BYTES`).
- `validateSarifResultCount()` caps the number of results at 10,000 (`MAX_SARIF_RESULTS`).

### T3 — Malformed SARIF JSON (parse crash)

**Threat:** Invalid JSON in a SARIF file could throw an unhandled exception.

**Mitigations:**
- `parseSarif()` wraps `JSON.parse` and the caller catches the exception, showing a user-friendly error.
- `validateSarifShape()` confirms the mandatory `version` and `runs` fields before processing.

### T4 — Oversized source file (in-editor analysis)

**Threat:** Analysing an extremely large Rust file could cause the extension to hang.

**Mitigations:**
- `MAX_SOURCE_BYTES` (10 MB) is checked before calling `analyzeSorobanSource()` in the export command.
- The VS Code text buffer already imposes a practical limit on files it opens.

### T5 — Sanctifier CLI binary substitution

**Threat:** If `sanctifier.sanctifierPath` is set to a malicious binary, the `analyzeWorkspace` command would execute it.

**Mitigations:**
- The path is supplied explicitly by the user via VS Code settings and is not derived from workspace content.
- Users are responsible for trusting the binary at the configured path (same model as other VS Code extensions that invoke external tools).

### T6 — Export path outside workspace

**Threat:** A crafted save dialog default URI could trick a user into overwriting files outside the workspace.

**Mitigations:**
- `isPathWithinWorkspace()` is called on the chosen destination path before writing.
- The save dialog starts from the directory of the active file, reducing confusion.

## Out of Scope

- Network-level threats (extension has no network calls).
- VS Code extension host sandbox escapes (OS / VS Code responsibility).
- Supply-chain attacks on npm packages (mitigated by lock-file pinning and `npm ci` in CI).

## Security Contact

Report vulnerabilities via the [HyperSafeD/Sanctifier GitHub Security Advisories](https://github.com/HyperSafeD/Sanctifier/security/advisories/new).
