# Analysis Profiles

The `--profile` flag selects a named preset that controls which findings are emitted and whether a non-zero exit code is returned. It overrides `--exit-code` and `--min-severity` when set.

```
sanctifier analyze --profile ci ./contracts
```

## Profile matrix

| Profile   | Rules emitted              | Fatal threshold       | Use case                            |
|-----------|----------------------------|-----------------------|-------------------------------------|
| `strict`  | All rules                  | Any finding           | Pre-merge gate on critical projects |
| `lenient` | Critical + High only       | None (always exits 0) | Developer workflow, noisy codebases |
| `audit`   | All rules                  | None (always exits 0) | Security audit reports              |
| `ci`      | All rules                  | Critical + High       | Standard CI pipeline gate           |

### strict

Emits all built-in rules and exits with code `1` if at least one finding is produced, regardless of severity.

```bash
sanctifier analyze --profile strict .
```

### lenient

Suppresses medium and low severity categories (storage collisions, variable shadowing, custom rules, contract-import mismatches, and vuln-db matches below High) to reduce noise during development. Always exits `0`.

```bash
sanctifier analyze --profile lenient .
```

### audit

Emits every rule including informational findings. Exits `0` regardless of findings — suitable for generating full audit reports where a non-zero exit would break tooling.

```bash
sanctifier analyze --profile audit --format json . > audit-report.json
```

### ci

Emits all rules so the full finding set is visible in logs, but only exits `1` when at least one **Critical** or **High** finding is present. Medium and Low findings are reported but non-fatal.

```bash
sanctifier analyze --profile ci --format json .
```

## Relationship to --exit-code / --min-severity

`--profile` takes precedence. When a profile is set, `--exit-code` and `--min-severity` are ignored.

To use custom thresholds without a preset, omit `--profile` and configure `--exit-code` with `--min-severity` directly:

```bash
# exit 1 on any medium-or-higher finding
sanctifier analyze --exit-code --min-severity medium .
```

## Profile in JSON output

The active profile is included in the `metadata` block of JSON reports:

```json
{
  "metadata": {
    "profile": "ci",
    ...
  }
}
```

When no profile is set, `"profile"` is `null`.
