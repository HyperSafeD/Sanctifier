# Mutation Testing Documentation

This document describes the mutation testing setup, results, and guidelines for Sanctifier.

## Overview

Mutation testing is a technique to evaluate the quality of test suites by introducing small changes (mutations) to the codebase and checking if the tests detect these changes. A high kill rate (>80%) indicates that tests are effective at catching bugs.

## Setup

### Tool

We use [`cargo-mutants`](https://github.com/sourcefrog/cargo-mutants) for Rust mutation testing.

### Installation

```bash
cargo install cargo-mutants
```

### Running Locally

To run mutation testing on `sanctifier-core`:

```bash
cd tooling/sanctifier-core
cargo mutants --no-shuffle --timeout 600
```

**Options:**
- `--no-shuffle`: Disable random mutation order for reproducibility
- `--timeout 600`: Set per-mutant timeout to 600 seconds (10 minutes)
- `--include-tests`: Also mutate test code (optional)
- `--file <pattern>`: Limit mutations to specific files

### Configuration

Mutation testing is configured in `.github/workflows/mutation-testing.yml`:

- **Schedule**: Runs weekly on Monday at 00:00 UTC
- **Manual trigger**: Available via `workflow_dispatch`
- **Non-blocking**: `continue-on-error: true` (expensive to run on every push)
- **Timeout**: 600 seconds per mutant
- **Target**: `tooling/sanctifier-core` package

## Results

### Latest Results

The most recent mutation testing run results are available as a GitHub Actions artifact (`mutation-test-report`).

### Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| **Kill Rate** | Percentage of mutants killed by tests | >80% |
| **Survived** | Mutants not detected by tests | Minimize |
| **Timeout** | Mutants that timed out during testing | Minimize |
| **Total Mutants** | Total number of mutations generated | N/A |

### Interpreting Results

- **Killed (caught)**: Tests failed when the mutation was introduced → ✅ Good
- **Survived**: Tests passed despite the mutation → ⚠️ Needs improvement
- **Timeout**: Mutation took too long to test → ⚠️ May need optimization

## Low-Kill-Rate Areas

Areas with low kill rates indicate insufficient test coverage. Common patterns:

### 1. Unreachable Code

Mutations in unreachable code paths will survive because tests don't exercise those paths.

**Example:**
```rust
fn process(data: &str) {
    if data.is_empty() {
        return;
    }
    // Only this path is tested
    process_non_empty(data);
}
// This function is never tested
fn process_empty() { ... }
```

**Fix:** Add tests for edge cases and error paths.

### 2. Complex Boolean Logic

Mutations in complex conditions may survive if tests don't cover all combinations.

**Example:**
```rust
if a && b || c && !d {
    // Complex condition
}
```

**Fix:** Use property-based testing or add explicit tests for each condition combination.

### 3. Error Handling

Mutations in error handling code often survive because tests focus on happy paths.

**Example:**
```rust
fn parse_config(path: &str) -> Result<Config, Error> {
    let content = fs::read_to_string(path)?;
    // Only happy path tested
    Ok(serde_json::from_str(&content)?)
}
```

**Fix:** Add tests for file not found, invalid JSON, permission errors, etc.

### 4. Z3-Dependent Code

Code that requires Z3 theorem prover may have lower coverage in environments without Z3.

**Fix:** Mock Z3 interactions or use conditional compilation for test environments.

## Improving Kill Rate

### Strategies

1. **Add Missing Tests**
   - Identify survived mutants
   - Write tests that would catch those mutations
   - Focus on edge cases and error paths

2. **Use Property-Based Testing**
   - Use `proptest` or `quickcheck` for complex logic
   - Generate random inputs to explore more code paths

3. **Increase Test Coverage**
   - Use `cargo tarpaulin` to identify untested code
   - Add tests for uncovered lines/branches

4. **Refactor Complex Code**
   - Break down complex functions into smaller, testable units
   - Reduce cyclomatic complexity

5. **Test Error Paths**
   - Add tests for all error conditions
   - Use `should_panic` or `Result::Err` assertions

### Example: Improving a Rule Test

**Before (low kill rate):**
```rust
#[test]
fn test_reentrancy_rule() {
    let contract = r#"
        fn transfer() {
            // Simple transfer
        }
    "#;
    let findings = analyze(contract);
    assert!(findings.is_empty());
}
```

**After (high kill rate):**
```rust
#[test]
fn test_reentrancy_rule_detects_vulnerability() {
    let contract = r#"
        fn transfer() {
            call_external();  // Mutant: remove this line
            update_balance(); // This should still be detected
        }
    "#;
    let findings = analyze(contract);
    assert!(findings.iter().any(|f| f.rule_id == "R001"));
}

#[test]
fn test_reentrancy_rule_safe_code() {
    let contract = r#"
        fn transfer() {
            update_balance();
            call_external();  // Safe: checks-effects-interactions
        }
    "#;
    let findings = analyze(contract);
    assert!(findings.is_empty());
}

#[test]
fn test_reentrancy_rule_multiple_calls() {
    let contract = r#"
        fn transfer() {
            call_external();
            call_external();  // Mutant: remove second call
            update_balance();
        }
    "#;
    let findings = analyze(contract);
    assert!(findings.len() >= 1);
}
```

## CI Integration

### Workflow File

See `.github/workflows/mutation-testing.yml` for the complete CI configuration.

### Artifacts

Mutation testing reports are uploaded as GitHub Actions artifacts:

- `mutants.out/`: Raw output data
- `mutants-report.html`: Human-readable HTML report

### PR Comments

When mutation testing runs on a PR, a summary comment is automatically posted:

```markdown
## Mutation Testing Results

- **Total Mutants**: 150
- **Killed**: 135
- **Survived**: 12
- **Timeout**: 3
- **Kill Rate**: 90.00%

✅ Kill rate meets target (>80%)

See the mutation-test-report artifact for detailed results.
```

## Best Practices

1. **Run Regularly**: Weekly runs catch regressions in test quality
2. **Review Survived Mutants**: Investigate each survived mutant to determine if it's a false positive or indicates missing tests
3. **Set Realistic Targets**: >80% kill rate is good, but 100% is often impractical
4. **Focus on Critical Code**: Prioritize mutation testing for security-critical rules and complex logic
5. **Don't Over-optimize**: Some survived mutants may be acceptable (e.g., trivial getters/setters)

## Troubleshooting

### Timeout Issues

If mutants frequently timeout:

- Increase timeout: `cargo mutants --timeout 1200`
- Optimize slow tests
- Use `--skip-tests` to skip known slow tests

### High Memory Usage

If mutation testing runs out of memory:

- Limit mutants: `cargo mutants --max-mutants 100`
- Run on smaller subsets: `cargo mutants --file path/to/specific/file.rs`

### Z3 Not Found

If Z3 is not available:

```bash
# Ubuntu/Debian
sudo apt-get install -y libz3-dev

# macOS
brew install z3

# Or skip Z3-dependent tests
cargo mutants --skip-tests z3
```

## References

- [cargo-mutants documentation](https://github.com/sourcefrog/cargo-mutants)
- [Mutation Testing Overview](https://en.wikipedia.org/wiki/Mutation_testing)
- [Sanctifier CI Workflow](.github/workflows/mutation-testing.yml)

## Contributing

When adding new rules or refactoring existing code:

1. Run mutation testing locally to verify test quality
2. Aim for >80% kill rate on new code
3. Document any intentional low-coverage areas
4. Update this document if mutation testing process changes

---

**Last Updated:** June 29, 2026