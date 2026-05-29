# Writing a Custom Rule in YAML — End-to-End Tutorial

This guide walks you through creating, validating, packaging, and sharing a custom
Sanctifier rule using the YAML rule format.

---

## Prerequisites

- Sanctifier CLI installed (`cargo install sanctifier` or see [Getting Started](../getting-started.md))
- A Soroban smart-contract project to lint
- Basic familiarity with YAML

---

## 1. Understanding the Rule Schema

A Sanctifier YAML rule has the following top-level fields:

```yaml
- id: <unique_snake_case_id>      # required — must be unique in the file
  name: <Human Readable Name>     # required
  description: <what it catches>  # required
  severity: error | warning | info
  matcher:
    type: <matcher_type>          # see §2 for available types
    # ... type-specific fields
```

Rules are stored in a file you reference from `.sanctify.toml`:

```toml
[rules]
custom = ["custom-rules.yaml"]
```

---

## 2. Worked Example — Detect `panic!` Inside `#[contractimpl]` Blocks

### 2.1 The problem

Soroban contracts must never call `panic!` directly in contract entry-points.
A panic unwinds the WASM host and gives no structured error to callers.
Use `Result<T, E>` and `return Err(...)` instead.

### 2.2 Write the rule

Create `custom-rules.yaml` in your project root:

```yaml
# custom-rules.yaml
- id: no_panic_in_contractimpl
  name: No panic! in #[contractimpl] blocks
  description: >
    Calling panic!() inside a #[contractimpl] block crashes the WASM host
    without a structured error. Return a typed Err(...) instead.
  severity: error
  matcher:
    type: regex
    pattern: 'panic!\s*\('
    scope: contractimpl   # only flag matches inside #[contractimpl] blocks
```

### 2.3 Register the rule

Add it to `.sanctify.toml`:

```toml
[rules]
custom = ["custom-rules.yaml"]
```

### 2.4 Run the linter

```bash
sanctifier check --manifest-path Cargo.toml
```

**Sample output** when a violation is found:

```
ERROR [no_panic_in_contractimpl] src/lib.rs:42:9
  No panic! in #[contractimpl] blocks
  | panic!("transfer failed");
  = help: return Err(ContractError::TransferFailed) instead
```

### 2.5 Suppress a specific occurrence

If a particular `panic!` is intentional, annotate the line:

```rust
panic!("unreachable"); // sanctifier: ignore[no_panic_in_contractimpl]
```

---

## 3. Available Matcher Types

| `type`              | Key fields                                         | Use for                              |
|---------------------|----------------------------------------------------|--------------------------------------|
| `regex`             | `pattern`, `scope?`                                | Raw text patterns                    |
| `function_call`     | `name`, `args?`                                    | Calls to a named free function       |
| `method_call`       | `method`, `receiver?`                              | Method calls (`obj.method(...)`)     |
| `storage_operation` | `operation` (`get`/`set`/`remove`), `key_pattern?` | DataStore read/write patterns        |

See [`custom-rules.example.yaml`](../custom-rules.example.yaml) for one example of each type.

---

## 4. Validation

Before committing, validate your rule file:

```bash
sanctifier rules validate custom-rules.yaml
```

Common validation errors:

| Error | Fix |
|-------|-----|
| `duplicate id` | Each `id:` must be unique across all loaded rule files |
| `unknown matcher type` | Check spelling — types are lowercase |
| `invalid severity` | Must be `error`, `warning`, or `info` |

---

## 5. Packaging for Reuse

### 5.1 Standalone rule repository

```
my-soroban-rules/
├── rules/
│   └── no-panic.yaml
└── README.md
```

Reference it from any project:

```toml
[rules]
remote = [
  { git = "https://github.com/your-org/my-soroban-rules", rev = "v1.0.0" }
]
```

### 5.2 Bundled with a crate

Add a `sanctifier/` directory to your crate:

```toml
[package.metadata.sanctifier]
rules = ["sanctifier/rules.yaml"]
```

Downstream users who add your crate automatically inherit the rules.

---

## 6. Sharing with the Community

1. Open a PR to [HyperSafeD/Sanctifier](https://github.com/HyperSafeD/Sanctifier) adding your rule to `custom-rules.example.yaml`
2. Include a test fixture in `tests/fixtures/` with a pass and fail case
3. Maintainers will review severity, description clarity, and matcher correctness

---

## Further Reading

- [`custom-rules.example.yaml`](../custom-rules.example.yaml) — full example rule set
- [`tooling/sanctifier-core/src/custom_yaml_rules.rs`](../tooling/sanctifier-core/src/custom_yaml_rules.rs) — rule engine source
- [Troubleshooting Guide](troubleshooting-guide.md)
- [Contributing](../CONTRIBUTING.md)
