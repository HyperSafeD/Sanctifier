# Mutation Testing Guide

## Overview

Mutation testing validates that tests actually catch bugs by introducing controlled mutations (artificial bugs) into the code and verifying that tests fail. This provides a much stronger quality signal than line coverage alone.

## Why Mutation Testing?

- **High line coverage doesn't guarantee quality tests**: You can have 100% line coverage with tests that never assert anything
- **Mutation testing verifies test effectiveness**: If a test suite doesn't catch a mutation (bug), there's a gap in testing
- **Mainnet stakes require confidence**: For production deployments, we need assurance that tests catch real bugs

## Threshold Requirements

### sanctifier-core
- **Minimum Kill Rate**: 75%
- **Target Kill Rate**: 80%+
- **Enforcement**: On release tags and release branches (blocking)
- **Informational**: On regular PRs (non-blocking, reported in comments)

### Tool
We use [`cargo-mutants`](https://mutants.rs/), a mutation testing tool specifically designed for Rust.

## Running Locally

### Quick Run (5-10 minutes)
```bash
cd tooling/sanctifier-core
cargo mutants --no-shuffle --timeout 60
```

### Full Run (30-60 minutes)
```bash
cd tooling/sanctifier-core
cargo mutants --no-shuffle --timeout 600
```

### Target Specific Files
```bash
cargo mutants --file src/parser/lexer.rs
```

### Generate HTML Report
```bash
cargo mutants --no-shuffle --output mutants.out
# View mutants.out/mutants-report.html in browser
```

## Interpreting Results

Mutation testing introduces changes to your code and runs your test suite. Each mutant falls into one of these categories:

### ✅ Caught
The test suite detected the mutation (test failed). This is good!

**Example:**
```rust
// Original
fn is_valid(x: u32) -> bool {
    x > 0
}

// Mutant: Changed > to >=
fn is_valid(x: u32) -> bool {
    x >= 0  // Bug introduced
}

// If test suite has:
assert!(!is_valid(0));  // This test will catch the mutant
```

### ⚠️ Survived
The mutation was not detected (all tests still passed). This indicates a test gap.

**Example:**
```rust
// Original
fn calculate_fee(amount: u64) -> u64 {
    amount / 100
}

// Mutant: Changed / to *
fn calculate_fee(amount: u64) -> u64 {
    amount * 100  // Bug introduced
}

// If no test verifies the fee calculation result, mutant survives
```

### ⏱️ Timeout
Test suite took too long to complete (exceeded timeout). May indicate:
- Performance issue introduced by mutation
- Infinite loop
- Test suite is generally slow

### ℹ️ Unviable
Mutation caused compile error. This is expected and not counted against kill rate.

## Improving Kill Rate

### 1. Add Tests for Boundary Conditions
```rust
#[test]
fn test_boundary_values() {
    assert!(validate_size(0));     // Minimum
    assert!(validate_size(MAX));   // Maximum
    assert!(!validate_size(MAX + 1)); // Just over
}
```

### 2. Test Error Paths
```rust
#[test]
fn test_invalid_input_returns_error() {
    let result = parse_source(&[0xFF, 0xFE]);
    assert!(matches!(result, Err(ParseError::InvalidUtf8)));
}
```

### 3. Verify Return Values Are Used
```rust
#[test]
fn test_calculation_correctness() {
    let fee = calculate_fee(1000);
    assert_eq!(fee, 10); // Don't just call it, verify the result!
}
```

### 4. Add Negative Test Cases
```rust
#[test]
fn test_should_reject_invalid_state() {
    let result = transition_to_invalid_state();
    assert!(result.is_err());
}
```

## CI Integration

### Regular PRs
Mutation testing runs on schedule (weekly) and is informational only. Results are posted as PR comments if run manually.

### Release Candidates
For release tags (`v*`) and release branches (`release/**`), mutation testing is **blocking**:
- Kill rate < 75%: CI fails ❌
- Kill rate ≥ 75%: CI passes ✅

See `.github/workflows/mutation-testing.yml` for configuration.

## Common Patterns and Solutions

### Pattern: Boolean Logic Mutations
**Mutation**: `>` changed to `>=`, `&&` changed to `||`
**Solution**: Test boundary conditions explicitly

### Pattern: Arithmetic Mutations
**Mutation**: `+` changed to `-`, `*` changed to `/`
**Solution**: Assert on specific calculation results

### Pattern: Return Value Mutations
**Mutation**: Return value changed from `Some(x)` to `None`
**Solution**: Test both success and failure paths

### Pattern: Constant Mutations
**Mutation**: Constant value changed (e.g., `100` to `101`)
**Solution**: Use known inputs with expected outputs

## Resources

- [cargo-mutants documentation](https://mutants.rs/)
- [Mutation Testing in Rust blog post](https://blog.rust-lang.org/inside-rust/2023/08/29/testing-the-test-suite.html)
- [Academic background on mutation testing](https://en.wikipedia.org/wiki/Mutation_testing)

## Troubleshooting

### "Too many mutants, taking too long"
Use `--file` to focus on specific files, or `--timeout` to reduce wait time.

### "All mutants timeout"
Your test suite may be slow. Consider:
- Optimizing slow tests
- Increasing `--timeout` value
- Using `--jobs` for parallelization

### "Kill rate seems low but tests are comprehensive"
Review surviving mutants individually - some may be in:
- Logging code
- Debug assertions
- Dead code paths
- Code that should be refactored

---

**Last Updated**: 2026-07-27
