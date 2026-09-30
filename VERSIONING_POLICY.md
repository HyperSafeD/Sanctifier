# Versioning Policy

**Applies to:** `sanctifier-cli`, `sanctifier-core`, and all published artifacts once they reach mainnet-stable `v1.0.0`.

This project follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html). Once `v1.0.0` is released, the public API is defined as:

1. **CLI flags and exit codes** — every flag, subcommand, and exit code documented by `sanctifier --help`
2. **Output schemas** — `schemas/analysis-output.json` and `schemas/sarif-2.1.0.json`
3. **Rule identifiers and severities** — the `S001..S012` (and `Z001..Z014`) rule codes and their default severities
4. **Library API** — public items exported by `sanctifier-core` (types, traits, functions) that are `#[doc(hidden)]`-free

Anything not listed above (internal modules, private types, test helpers, example code) is **not** part of the public API and may change at any minor/patch version.

---

## 1. Major version (`X.0.0`)

A major release signals **breaking changes**. Consumers must expect to update their integration.

### What requires a major bump

| Area | Breaking change |
|------|----------------|
| **CLI flags** | Removing or renaming an existing flag or subcommand |
| **CLI flags** | Changing the default value of a flag in a way that alters behaviour |
| **CLI flags** | Changing the type or format of a flag's argument |
| **Exit codes** | Removing, renaming, or changing the meaning of an exit code |
| **Output schema** | Removing a required field from `schemas/analysis-output.json` |
| **Output schema** | Changing the type of an existing field |
| **Output schema** | Making an optional field required |
| **SARIF output** | Removing or renaming a property that Sanctifier populates in `schemas/sarif-2.1.0.json` |
| **Rules** | Removing a rule (`S001`, `S002`, etc.) entirely |
| **Rules** | Changing a rule's default severity from the existing classification (e.g. `error` → `warning`) |
| **Rules** | Renaming a rule identifier code |
| **Library API** | Removing or renaming any public function, type, trait, or constant |
| **Library API** | Changing function signatures (adding required params, changing param types, restricting trait bounds) |
| **Library API** | Changing the behaviour of a public function in a way that breaks existing callers |

### Examples

- Removing `--format json` and making JSON the implicit default → **major** (default behaviour changes)
- Renaming `S001` to `S100` → **major** (suppression files and dashboards break)
- Deleting the `findings` array from `analysis-output.json` → **major** (all consumers break)

---

## 2. Minor version (`X.Y.0`)

A minor release adds functionality **without breaking** any existing public API.

### What requires a minor bump

| Area | Non-breaking addition |
|------|----------------------|
| **CLI flags** | Adding a new flag or subcommand |
| **CLI flags** | Adding a new optional parameter to an existing subcommand |
| **Output schema** | Adding a new optional field to `analysis-output.json` |
| **Output schema** | Adding a new optional property to `sarif-2.1.0.json` output |
| **Rules** | Adding a new rule (`S013`, `S014`, etc.) |
| **Rules** | Adding a new severity level to the taxonomy |
| **Rules** | Adding new optional configuration for an existing rule |
| **Library API** | Adding new public functions, types, traits, or constants |
| **Library API** | Adding new default trait implementations |
| **Library API** | Widening accepted input types (e.g. `&[u8]` to `impl AsRef<[u8]>`) |

### Examples

- Adding `--timeout 30` flag → **minor**
- Adding `S013` rule for a new vulnerability class → **minor**
- Adding an `optional_warnings` array to `analysis-output.json` → **minor**

---

## 3. Patch version (`X.Y.Z`)

A patch release ships **bug fixes and performance improvements** that do not change the public API.

### What requires a patch bump

| Area | Bug fix / internal change |
|------|--------------------------|
| **CLI** | Fixing a crash, hang, or incorrect exit code (without changing the documented meaning of the code) |
| **Output** | Fixing a bug in finding detection that changes results without changing the schema |
| **Output** | Correcting a SARIF property value format that violates the spec |
| **Rules** | Narrowing false positives or expanding true positive coverage within the existing rule scope |
| **Rules** | Performance improvements to rule detection |
| **Library API** | Internal refactors, documentation improvements, dependency updates (no API surface change) |
| **Build** | CI/CD fixes, toolchain version bumps, packaging fixes |

### Examples

- Fixing a panic when analysing empty files → **patch**
- Reducing false-positive rate for `S003` unchecked arithmetic → **patch**
- Updating `serde_json` dependency → **patch**

---

## 4. Schema versioning (`schema_version`)

The `schema_version` field in `analysis-output.json` follows its **own** semver, independent of the tool version.

| Schema change | `schema_version` bump |
|---------------|-----------------------|
| Adding an optional field in a backward-compatible way | Minor |
| Removing / renaming a field, or changing a type | Major |
| Fixing a field description or example (no shape change) | Patch |

The `sarif-2.1.0.json` schema is the upstream SARIF 2.1.0 standard. Sanctifier populates a subset of its properties. Adding a **new** SARIF property that Sanctifier now populates is a minor change; stopping population of a previously-populated property is a major change.

---

## 5. Pre-1.0.0 exceptions

Before `v1.0.0` (current state), the project follows **zero-version** SemVer conventions:

- Minor version bumps (`0.1.0` → `0.2.0`) may include breaking changes without notice
- Patch version bumps (`0.1.0` → `0.1.1`) are bug fixes only
- Rule codes (`S001`–`S012`) are considered stable even before `v1.0.0` and will not be removed or renamed without a deprecation notice

---

## 6. Deprecation process

Before a breaking change is shipped in a major version:

1. The deprecated API is marked with a `#[deprecated]` attribute (library) or a `--deprecated` notice in `--help` output (CLI)
2. The deprecation notice specifies the planned removal version (e.g. "removed in v3.0.0")
3. At least **one minor version** passes between the deprecation notice and the removal
4. The deprecation is documented in `CHANGELOG.md` under a `### Deprecated` section

---

## 7. Exit code stability

Exit codes are part of the public API and follow the same major/minor rules:

| Exit code | Meaning | Stability |
|-----------|---------|-----------|
| `0` | Success, no findings | Stable |
| `1` | Analysis completed with findings | Stable |
| `2` | Analysis error (timeout, parse error, I/O error) | Stable |
| `3` | Configuration error (invalid flags, missing file) | Stable |
| `4` | Internal error (bug, invariant violation) | Stable |
| `5`–`127` | Reserved for future use | Adding new codes = minor |

Exit codes `128+` are reserved for OS signal handling and must not be used by Sanctifier.

---

## 8. Version alignment

All published artifacts that share the `sanctifier` name **must** carry the same version number at any given release:

- `sanctifier-cli` (crates.io / npm / Homebrew)
- `sanctifier-core` (crates.io)
- `sanctifier-detector` (crates.io)
- `sanctifier-wasm` (crates.io)
- `vscode-extension` (VS Code Marketplace)

The version is bumped atomically by `scripts/release.sh` across all manifests. No artifact is published individually without the others being updated.

---

## 9. Enforcement & automation

- CI checks that the `schema_version` in `schemas/analysis-output.json` is bumped appropriately for the diff
- `scripts/validate_release_artifacts.js` verifies version consistency across all manifests
- The release-gate Action (#1189) blocks tag creation if any version string is inconsistent
- `CHANGELOG.md` must have an entry for the new version before a release tag is created

---

## References

- [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)
- `schemas/analysis-output.json` — Sanctifier analysis output schema
- `schemas/sarif-2.1.0.json` — SARIF output schema
- `CHANGELOG.md` — release history
- `scripts/release.sh` — version bump script
- `scripts/validate_release_artifacts.js` — pre-release validation
