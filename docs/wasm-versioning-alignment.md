# WASM Module Versioning & Input Validation

## Overview

This document describes the versioning alignment and input validation hardening implemented in `tooling/sanctifier-wasm` to ensure production-ready behavior and safe-by-default operation.

## Versioning Strategy

### Module Versions

- **sanctifier-wasm**: `0.2.0` (independent semantic versioning)
- **sanctifier-cli**: `0.1.0` (independent semantic versioning)
- **sanctifier-core**: `0.1.0` (independent semantic versioning)

Each module maintains its own semantic version to reflect independent release cycles and feature maturity.

### Schema Version

The analysis output schema is versioned independently via the `schema_version` field:

```json
{
  "findings": [...],
  "summary": {...},
  "schema_version": "1.0.0"
}
```

**Current Schema Version**: `1.0.0`

This allows:

- Breaking changes to output format to be tracked separately from tool versions
- Consumers to validate compatibility without checking tool version
- Gradual migration paths when schema changes occur

## Input Validation

### Source Code Validation

All WASM analysis functions validate source code input:

- **Minimum size**: 1 byte (non-empty)
- **Maximum size**: 10 MB (prevents memory exhaustion)

Error response on validation failure:

```json
{
  "error_code": "INVALID_INPUT",
  "message": "Source code exceeds maximum size of 10485760 bytes (got X bytes)",
  "schema_version": "1.0.0"
}
```

### Configuration Validation

The `analyze_with_config()` function validates JSON configuration:

- **Minimum size**: Non-empty after trimming
- **Maximum size**: 1 MB (prevents memory exhaustion)

Error response on validation failure:

```json
{
  "error_code": "INVALID_CONFIG",
  "message": "Configuration JSON cannot be empty",
  "schema_version": "1.0.0"
}
```

### Error Codes

| Code             | Meaning                         | Recovery                     |
| ---------------- | ------------------------------- | ---------------------------- |
| `INVALID_INPUT`  | Source code validation failed   | Check source size and format |
| `INVALID_CONFIG` | Configuration validation failed | Verify JSON syntax and size  |
| `PARSE_ERROR`    | Internal parsing error          | Retry or report issue        |

## API Changes

### New Functions

#### `schema_version() -> String`

Returns the analysis output schema version (e.g., `"1.0.0"`).

```javascript
const schemaVer = sanctifier.schema_version();
console.log(schemaVer); // "1.0.0"
```

### Updated Functions

#### `analyze(source: &str) -> JsValue`

Now validates source code before analysis:

```javascript
const result = sanctifier.analyze(sourceCode);

// Check for error response
if (result.error_code) {
  console.error(`Error: ${result.message}`);
} else {
  console.log(`Found ${result.summary.total} issues`);
}
```

#### `analyze_with_config(config_json: &str, source: &str) -> JsValue`

Now validates both configuration and source code:

```javascript
const config = JSON.stringify({ ledger_limit: 64000 });
const result = sanctifier.analyze_with_config(config, sourceCode);

if (result.error_code) {
  console.error(`Error: ${result.message}`);
}
```

### Updated Output Format

All `AnalysisResult` objects now include `schema_version`:

```json
{
  "findings": [
    {
      "code": "S001",
      "category": "authentication",
      "message": "Missing authentication guard in `initialize`",
      "location": "initialize"
    }
  ],
  "summary": {
    "total": 1,
    "auth_gaps": 1,
    "panic_issues": 0,
    "arithmetic_issues": 0,
    "size_warnings": 0,
    "unsafe_patterns": 0,
    "storage_collisions": 0,
    "event_issues": 0,
    "unhandled_results": 0,
    "upgrade_risks": 0,
    "sep41_issues": 0,
    "has_critical": false,
    "has_high": true
  },
  "schema_version": "1.0.0"
}
```

## Migration Guide

### For JavaScript Consumers

1. **Check for errors first**:

   ```javascript
   const result = sanctifier.analyze(code);
   if (result.error_code) {
     // Handle error
     console.error(result.message);
     return;
   }
   ```

2. **Use schema_version for compatibility**:

   ```javascript
   if (result.schema_version !== "1.0.0") {
     console.warn("Incompatible schema version");
   }
   ```

3. **Validate input before calling**:
   ```javascript
   if (!code || code.length === 0) {
     console.error("Source code cannot be empty");
     return;
   }
   if (code.length > 10 * 1024 * 1024) {
     console.error("Source code too large");
     return;
   }
   ```

### For Rust Consumers

The WASM module is primarily designed for JavaScript/browser use. For Rust consumers, use `sanctifier-core` directly:

```rust
use sanctifier_core::{Analyzer, SanctifyConfig};

let analyzer = Analyzer::new(SanctifyConfig::default());
let findings = analyzer.scan_auth_gaps(source_code);
```

## Performance Considerations

### Large Contract Handling

The WASM module is optimized for contracts up to 10 MB:

- **Typical contract** (< 100 KB): < 100ms
- **Large contract** (1-5 MB): 500ms - 2s
- **Very large contract** (5-10 MB): 2-5s

For contracts exceeding 10 MB, use the CLI tool instead:

```bash
sanctifier analyze ./large-contract --timeout 120
```

### Benchmarking

Run benchmarks with:

```bash
cd tooling/sanctifier-core
cargo bench
```

## Testing

### Unit Tests

```bash
cd tooling/sanctifier-wasm
cargo test --lib
```

### WASM Tests

```bash
cd tooling/sanctifier-wasm
wasm-pack test --headless --firefox
```

### Integration Tests

```bash
cd tooling/sanctifier-cli
cargo test --test versioning_tests
```

## Backward Compatibility

### Breaking Changes

- `AnalysisResult` now includes `schema_version` field
- Error responses use new `ErrorResponse` type
- Input validation may reject previously accepted inputs

### Migration Path

1. Update consumers to handle `schema_version` field
2. Add error handling for validation failures
3. Test with new version before deploying

## Future Versions

### Schema Version 2.0.0 (Planned)

- Additional finding metadata (severity levels, remediation steps)
- Structured error responses with error codes
- Performance metrics in summary

### Module Version 0.3.0 (Planned)

- Streaming analysis for very large contracts
- Custom rule support via WASM
- Formal verification integration

## References

- [Semantic Versioning](https://semver.org/)
- [JSON Schema](https://json-schema.org/)
- [WASM Bindings](https://rustwasm.org/docs/wasm-bindgen/)


// Comment for starting on the task 