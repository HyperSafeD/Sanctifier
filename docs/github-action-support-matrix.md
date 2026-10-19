# GitHub Action Support Matrix

This document describes the supported environment for the composite GitHub Action in `action.yml`.

## Supported runners

- **ubuntu-latest**: supported
- **macos-latest**: supported
- **windows-latest**: supported (best-effort; ensure the CLI can be installed for your target environment)

## Compatibility matrix

| Surface        | Supported values                                  | Validation owner           | Failure behavior                                                              |
| -------------- | ------------------------------------------------- | -------------------------- | ----------------------------------------------------------------------------- |
| Runner OS      | `ubuntu-latest`, `macos-latest`, `windows-latest` | `.github/workflows/ci.yml` | CI job fails on unsupported environment regressions.                          |
| `path`         | Existing relative path inside the checkout        | `scripts/action_inputs.py` | Emits `Invalid Input` with a path-specific message.                           |
| `format`       | `text`, `json`, `sarif`                           | `scripts/action_inputs.py` | Emits allowed values and the rejected value.                                  |
| `min-severity` | `critical`, `high`, `medium`, `low`, `info`       | `scripts/action_inputs.py` | Emits allowed values and the rejected value.                                  |
| `upload-sarif` | `true`, `false`, `1`, `0`, `yes`, `no`            | `scripts/action_inputs.py` | Emits a boolean-normalization error.                                          |
| `sarif-output` | Safe relative output path                         | `scripts/action_inputs.py` | Rejects absolute paths, traversal, control characters, and unsafe characters. |
| `debug`        | `true`, `false`, `1`, `0`, `yes`, `no`            | `scripts/action_inputs.py` | Emits a boolean-normalization error.                                          |

## Required tools on the runner

The action installs the Sanctifier CLI via `cargo install`, so the runner needs:

- **Rust toolchain** (via `rustup` / `cargo` in PATH)
- **Network access** to download Rust crates

If you use Sanctifier’s formal-verification features, your project may also require:

- **Z3** headers/libraries (platform-specific)

## GitHub token permissions

The action itself does not require elevated permissions to _run_ a scan.

- **Minimum** (scan only): `contents: read`
- **SARIF upload enabled** (`format: sarif` + `upload-sarif: "true"`): `security-events: write`

Notes:

- The action intentionally skips SARIF upload on `pull_request` events originating from forks.
- If you disable SARIF upload (`upload-sarif: "false"`), you should omit `security-events: write`.

## Inputs stability

The action inputs/outputs in `action.yml` are intended to remain stable across patch releases.
If an input/output needs to change incompatibly, the change should be documented and released with an appropriate version bump.

## Input validation and error messages

The composite action validates its inputs before installing or running the CLI:

- `path` must be a relative path inside the checked-out repository and must exist.
- `format` must be one of `text`, `json`, or `sarif`.
- `min-severity` must be one of `critical`, `high`, `medium`, `low`, or `info`.
- `upload-sarif` accepts boolean-like values and is normalized to `true` or `false`.
- `sarif-output` must be a relative output path and cannot include path traversal segments.
- `debug` accepts boolean-like values and is normalized to `true` or `false`.

Invalid inputs fail before installation or scanning begins. The helper emits GitHub annotation syntax with this stable prefix:

```text
::error title=Invalid Input::Sanctifier action input error:
```

Contributor-facing messages should name the invalid input, explain the accepted shape, and include the rejected value when that is safe. Do not include secrets or dump full environment values in validation errors.

Action helper logic lives in `scripts/action_inputs.py`, `scripts/action_summary.py`, and `scripts/resolve_action_version.py`.
Unit fixtures live under `tests/action/fixtures/`, and CI runs them with:

```bash
python -m unittest discover -s tests/action -p "test_*.py"
```

## Output stability

The support matrix documents existing action behavior and does not change Sanctifier CLI, SARIF, JSON, schema, or contract output formats. If a future compatibility update changes an action input, output, or error prefix incompatibly, include a version bump and migration note before updating the matrix.

## Debug logging mode

The composite action supports a safe-by-default debug logging mode:

- Set `with: debug: "true"` to print extra `[sanctifier-action][debug] ...` lines.
- Debug logs include normalized inputs, resolved CLI version, and scan parameters.
- Debug logs must **not** print secrets or dump the full environment.

### Contributor notes

- Keep debug output stable and parseable (`[sanctifier-action][debug]` prefix).
- Prefer adding new debug details rather than changing existing fields in-place.
