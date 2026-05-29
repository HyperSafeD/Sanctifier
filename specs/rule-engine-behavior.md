# Rule Engine Behavior Specification

> Defines the expected runtime behavior of Sanctifier's rule engine: how rules are
> discovered, prioritized, matched, and reported.

---

## 1. Rule Discovery Order

When `sanctifier analyze` is invoked, the engine loads rules in this order:

1. **Built-in rules** compiled into `sanctifier-core` (codes `S001`–`S012`).
2. **Inline custom rules** from `[[custom_rules]]` entries in `.sanctify.toml`.
3. **YAML custom rules** from the file referenced by `custom_rules_yaml` in
   `.sanctify.toml`.

Rules loaded later **do not** override earlier rules. All loaded rules run
independently and produce separate findings.

## 2. Rule Filtering

### `enabled_rules`

- When `enabled_rules` is **present and non-empty**, only the listed built-in
  rule keys are active. Unlisted built-in rules are skipped.
- When `enabled_rules` is **absent**, all built-in rules are active.
- Custom rules (`[[custom_rules]]` and YAML) are **always active** and are not
  affected by `enabled_rules`.

### `strict_mode`

- When `strict_mode = true`:
  - All built-in rules are active regardless of `enabled_rules`.
  - Severity levels cannot be downgraded by configuration.
- When `strict_mode = false` (default):
  - `enabled_rules` is respected.
  - Future: per-rule severity overrides may be supported.

### `ignore_paths`

- Glob patterns relative to the project root.
- Files matching any `ignore_paths` entry are excluded from analysis.
- The `target/` directory is **always** excluded, even if not listed.

## 3. Matching Semantics

### Built-in Rules

Built-in rules use AST-level analysis within `sanctifier-core`. They operate on
parsed Rust source and produce structured findings with precise source locations
(file, line, column).

### Inline Custom Rules (TOML)

Inline rules use **regex matching** over raw source text:

- The `pattern` field is compiled as a Rust regex.
- Backslashes must be double-escaped in TOML (`\\s` for `\s`).
- Matches produce findings with code `S007` and the rule's `name` in the message.
- Each match reports the first line of the matched region.

### YAML Custom Rules

YAML rules support **structured matchers** (`function_call`, `storage_operation`,
`method_call`, `regex`):

- `function_call`: matches by function name. `args` is reserved for future use.
- `storage_operation`: matches `set`/`get`/`remove` operations with an optional
  `key_pattern` glob.
- `method_call`: matches method name with optional `receiver` glob.
- `regex`: equivalent to inline TOML rules but in YAML format.

All YAML rule findings are also reported with code `S007`.

## 4. Finding Output Format

Each finding is a JSON object with this stable shape:

```json
{
  "code": "S001",
  "severity": "error",
  "message": "Missing require_auth in state-changing function",
  "file": "src/contract.rs",
  "line": 42,
  "column": 5,
  "rule_id": "auth_gaps"
}
```

### Guarantees

- The `code` field always matches pattern `S0[0-9]{2}`.
- The `severity` field is always one of `error`, `warning`, `info`.
- The `file` path is relative to the project root.
- `line` and `column` are 1-indexed.
- Custom rules always use `code: "S007"`.

### SARIF Compatibility

The JSON output is designed to be convertible to SARIF v2.1.0. The `code` field
maps to `ruleId`, and `severity` maps to the SARIF `level` property.

## 5. Execution Order and Deduplication

- Rules execute in discovery order (built-in → inline TOML → YAML).
- If two rules produce a finding at the **exact same file + line + column + code**,
  only the first is kept (deduplication).
- Findings are sorted in the final output by: file (ascending) → line (ascending)
  → code (ascending).

## 6. Exit Codes

| Code | Meaning                                      |
| ---- | -------------------------------------------- |
| `0`  | Analysis succeeded, no `error`-severity findings |
| `1`  | Analysis succeeded, at least one `error`-severity finding |
| `2`  | Analysis failed (config error, I/O error, etc.)  |

- `warning` and `info` findings do **not** affect the exit code.
- In `strict_mode`, any finding (regardless of severity) causes exit code `1`.

---

*Last updated: April 2026*
