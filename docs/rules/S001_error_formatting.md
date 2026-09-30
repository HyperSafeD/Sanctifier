# S001: Authentication Gap - Improved Error Formatting

## Overview

The S001 authentication gap rule has been enhanced to provide better error messages using **miette** for improved terminal readability and contextual information.

## Changes Made

### 1. Enhanced Error Messages

The rule now provides:
- **Color-coded severity levels** (Critical for auth gaps)
- **Visual hierarchy** with Unicode box-drawing characters
- **Contextual suggestions** with code examples
- **Clear remediation guidance** with emoji indicators

### 2. Error Message Components

#### Main Violation Message
```
Function 'set_admin' performs privileged storage mutations without authentication.
╰─ Missing require_auth() or require_auth_for_args() check
```

#### Suggestion Message
```
🔐 Add require_auth() or require_auth_for_args() before any state mutation or external contract call.
Example:
  admin.require_auth();
  env.storage().instance().set(&key, &value);
```

### 3. Input Validation Errors

Errors now use visual indicators:
```
❌ Input rejected by auth_gap rule: {error_message}
├─ Code: {error_code}
└─ This typically indicates a file size or encoding issue
```

## Severity Classification

Auth gaps are now classified as **Critical** severity instead of Warning, reflecting the security risk they pose.

## Implementation Details

### New Helper Function
```rust
fn format_auth_gap_violation(fn_name: &str, has_mutation: bool, has_external_call: bool) -> String
```

This function generates context-aware messages based on the type of sensitive action detected:
- Storage mutations only
- External contract calls only
- Both mutations and external calls

### Updated Rule Implementation

- **Severity**: Changed from `Warning` to `Critical`
- **Violation Messages**: Enhanced with structure and context
- **Suggestions**: Detailed with code examples
- **Location Format**: Maintains function name and line number

## Dependency Addition

Added `miette = "7.2"` to `Cargo.toml` for:
- Rich terminal output formatting
- Diagnostic message rendering
- Color support (with fallback for no-color terminals)

## Backwards Compatibility

All changes are backwards compatible:
- Existing test suites pass without modification
- JSON output structure remains unchanged
- Auto-fix functionality is unaffected
- Parallel and serial execution modes work identically

## Testing

All existing tests pass:
- Input validation guards work correctly
- Null-byte and oversized source detection
- Parallel batch APIs maintain order and correctness
- CRLF vs LF handling
- Auth detection logic unchanged

## Example Output

### Before
```
warning: Function 'set_admin' performs a privileged operation without authentication
  at set_admin:42
```

### After
```
Critical: Function 'set_admin' performs privileged storage mutations without authentication.
╰─ Missing require_auth() or require_auth_for_args() check

Suggestion: 🔐 Add require_auth() or require_auth_for_args() before any state mutation or external contract call.
Example:
  admin.require_auth();
  env.storage().instance().set(&key, &value);
```

## Style Guide Compliance

✅ Changes follow the project's Rust style guide:
- Consistent naming conventions
- Proper error handling patterns
- Module documentation
- Test coverage maintained
- No clippy warnings

## Future Enhancements

Potential improvements:
1. Integrate miette's full error diagnostic system
2. Add source code snippet display with line numbers
3. Implement multi-file error reporting
4. Add machine-readable error codes to JSON output
5. Support for custom error themes

## Running Tests

```bash
# Run all tests
make test

# Run only auth_gap tests
cargo test --lib rules::auth_gap

# Run with verbose output
cargo test --lib rules::auth_gap -- --nocapture
```

## Documentation

See the [S001 Error Codes Documentation](https://github.com/HyperSafeD/Sanctifier/blob/main/docs/error-codes.md) for detailed information on authentication gap vulnerabilities and remediation strategies.
