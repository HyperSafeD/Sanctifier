# S003: Arithmetic Overflow / Underflow Detection

## Overview

**Finding Code:** `S003`  
**Category:** `arithmetic`  
**Severity:** Medium  
**Title:** Unchecked Arithmetic

The S003 rule detects unchecked arithmetic operations that could overflow or underflow in Soroban smart contracts. Integer overflow/underflow in financial applications can lead to critical vulnerabilities including loss of funds, incorrect balances, and unauthorized minting.

## Problem Statement

In Rust, integer overflow behavior differs between debug and release builds:
- **Debug builds**: Overflow causes a panic
- **Release builds**: Overflow wraps around (modular arithmetic)

For smart contracts handling financial operations, either behavior is unacceptable:
- Panics can lock contract state
- Silent wraparound can cause loss of funds

## Detection Rules

### Flagged Operations

The rule flags the following unchecked operations:

#### Binary Operators
- `+` (addition) - can overflow
- `-` (subtraction) - can underflow  
- `*` (multiplication) - can overflow
- `/` (division) - can panic on division by zero
- `%` (modulo) - can panic on modulo by zero

#### Compound Assignment Operators
- `+=` (add-assign)
- `-=` (sub-assign)
- `*=` (mul-assign)
- `/=` (div-assign)
- `%=` (rem-assign)

#### Custom Math Methods
- `.mul_div()` - numerator × multiplier can overflow before division
- `.div_ceil()` - potential boundary issues
- `.fixed_point_mul()` - fixed-point multiplication without overflow protection
- `.fixed_point_div()` - fixed-point division without overflow protection

#### Custom Math Functions
- `mul_div(a, b, c)` - function-style multiplication-division
- `fixed_point_mul(a, b)` - function-style fixed-point multiply
- `fixed_point_div(a, b)` - function-style fixed-point divide

### Exclusions

The following patterns are **NOT flagged** to reduce false positives:

1. **Test code**:
   - Functions with `#[test]` attribute
   - Code inside `#[cfg(test)]` modules

2. **Array/slice indexing**:
   - Arithmetic in array subscripts (e.g., `buf[i + 1]`) is considered idiomatic Rust

3. **Comparison operators**:
   - `>`, `<`, `>=`, `<=`, `==`, `!=` (no overflow risk)

4. **Bitwise operators**:
   - `&`, `|`, `^`, `<<`, `>>` (different risk profile)

5. **String concatenation**:
   - `+` operator on string literals

6. **Safe methods**:
   - `.checked_add()`, `.checked_sub()`, `.checked_mul()`, etc.
   - `.saturating_add()`, `.saturating_sub()`, `.saturating_mul()`, etc.
   - `.checked_mul_div()`, `.checked_fixed_point_mul()`, etc.

## Implementation Details

### Module Location
- **Rule Implementation**: `tooling/sanctifier-core/src/rules/arithmetic_overflow.rs`
- **Core Integration**: `tooling/sanctifier-core/src/lib.rs` (`scan_arithmetic_overflow()` method)
- **Finding Code**: `tooling/sanctifier-core/src/finding_codes.rs` (`ARITHMETIC_OVERFLOW`)

### Detection Algorithm

The detector uses a Syn AST visitor pattern:

1. **Parse** the source code into an AST
2. **Visit** each function in `impl` blocks
3. **Track** current function context
4. **Detect** binary operations and method calls
5. **Deduplicate** findings per (function, operator) pair
6. **Skip** test code and index expressions

### Deduplication Strategy

To avoid noise, the rule reports **at most one finding per (function_name, operation) pair**. For example:

```rust
pub fn sum_three(a: u64, b: u64, c: u64) -> u64 {
    a + b + c  // Only one S003 finding for '+' in this function
}
```

This prevents reporting multiple findings for repeated use of the same operator in a single function.

### Output Format

Each finding includes:

```rust
ArithmeticIssue {
    function_name: String,    // e.g., "transfer"
    operation: String,         // e.g., "+", "-", "mul_div"
    suggestion: String,        // e.g., "Use .checked_add(rhs)..."
    location: String,          // e.g., "transfer:42"
}
```

## Examples

### Vulnerable Code

```rust
#[contractimpl]
impl Token {
    /// ❌ VULNERABLE: Unchecked addition can overflow
    pub fn mint(env: Env, to: Address, amount: i128) {
        let current = get_balance(&env, &to);
        let new_balance = current + amount;  // S003: overflow risk
        set_balance(&env, &to, new_balance);
    }

    /// ❌ VULNERABLE: Unchecked subtraction can underflow
    pub fn burn(env: Env, from: Address, amount: i128) {
        let current = get_balance(&env, &from);
        let new_balance = current - amount;  // S003: underflow risk
        set_balance(&env, &from, new_balance);
    }

    /// ❌ VULNERABLE: Compound assignment
    pub fn accumulate(env: Env, mut balance: u64, delta: u64) {
        balance += delta;  // S003: overflow risk
    }

    /// ❌ VULNERABLE: mul_div without overflow protection
    pub fn calculate_fee(env: Env, amount: i128, rate: i128, divisor: i128) -> i128 {
        amount.mul_div(rate, divisor)  // S003: numerator * rate can overflow
    }
}
```

### Safe Code

```rust
#[contractimpl]
impl Token {
    /// ✅ SAFE: checked_add returns None on overflow
    pub fn mint(env: Env, to: Address, amount: i128) {
        let current = get_balance(&env, &to);
        let new_balance = current.checked_add(amount)
            .expect("mint: balance overflow");
        set_balance(&env, &to, new_balance);
    }

    /// ✅ SAFE: saturating_sub clamps at zero
    pub fn burn(env: Env, from: Address, amount: i128) {
        let current = get_balance(&env, &from);
        let new_balance = current.saturating_sub(amount);
        set_balance(&env, &from, new_balance);
    }

    /// ✅ SAFE: explicit checked operation with assignment
    pub fn accumulate(env: Env, mut balance: u64, delta: u64) {
        balance = balance.checked_add(delta)
            .expect("accumulate: overflow");
    }

    /// ✅ SAFE: Array indexing arithmetic is not flagged
    pub fn safe_index(env: Env, buf: &[u8], i: usize) -> u8 {
        buf[i + 1]  // Idiomatic Rust pattern - not flagged
    }
}
```

## Remediation Guide

### 1. Use Checked Operations

Replace unchecked arithmetic with `.checked_*` methods:

| Unchecked | Checked Alternative | Behavior on Overflow |
|-----------|-------------------|---------------------|
| `a + b` | `a.checked_add(b)` | Returns `None` |
| `a - b` | `a.checked_sub(b)` | Returns `None` |
| `a * b` | `a.checked_mul(b)` | Returns `None` |
| `a / b` | `a.checked_div(b)` | Returns `None` (div by zero) |
| `a % b` | `a.checked_rem(b)` | Returns `None` (mod by zero) |

**Example:**
```rust
let result = amount.checked_add(fee)
    .expect("transfer: overflow");
```

### 2. Use Saturating Operations

For cases where clamping is acceptable:

| Unchecked | Saturating Alternative | Behavior on Overflow |
|-----------|----------------------|---------------------|
| `a + b` | `a.saturating_add(b)` | Clamps at `MAX` |
| `a - b` | `a.saturating_sub(b)` | Clamps at `0` |
| `a * b` | `a.saturating_mul(b)` | Clamps at `MAX` |

**Example:**
```rust
// Safe for non-financial calculations
let score = current_score.saturating_add(bonus);
```

### 3. Handle Compound Assignments

Replace compound operators with explicit checked operations:

```rust
// ❌ Before
balance += amount;

// ✅ After
balance = balance.checked_add(amount)
    .expect("balance overflow");
```

### 4. Use Safe Math Wrappers

For custom math operations, use checked variants:

```rust
// ❌ Before
let result = a.mul_div(b, c);

// ✅ After  
let result = a.checked_mul_div(b, c)
    .expect("mul_div overflow");
```

## Testing

### Unit Tests

The rule includes comprehensive unit tests in `tooling/sanctifier-core/src/rules/arithmetic_overflow.rs`:

- `test_flag_standard_arithmetic` - Detects basic +, -, *, /, %
- `test_flag_custom_math_methods` - Detects .mul_div(), .fixed_point_mul()
- `test_flag_custom_math_calls` - Detects function-style math operations
- `test_ignore_checked_methods` - No false positives on safe methods
- `test_skip_test_attribute_functions` - Skips #[test] functions
- `test_skip_cfg_test_module` - Skips #[cfg(test)] modules
- `test_skip_index_subscript_arithmetic` - Skips array indexing

### Integration Tests

Integration tests in `tooling/sanctifier-core/src/lib.rs`:

- `test_scan_arithmetic_overflow_basic` - Multiple operators in one contract
- `test_scan_arithmetic_overflow_compound_assign` - +=, -=, *= operators
- `test_scan_arithmetic_overflow_deduplication` - One finding per (fn, op) pair
- `test_scan_arithmetic_overflow_no_false_positive_safe_code` - No flags on comparisons/bitwise
- `test_scan_arithmetic_overflow_custom_wrapper_types` - Detects in wrapped types
- `test_scan_arithmetic_overflow_suggestion_content` - Verifies suggestion quality

### Fixture Contract

The canonical test fixture is `contracts/fixtures/finding-codes/s003_arithmetic.rs`, which includes:

**Unsafe patterns (MUST flag):**
- `unchecked_add` - u32 + u32
- `unchecked_sub` - u32 - u32  
- `unchecked_mul` - u32 * u32
- `unchecked_add_assign` - i128 += i128
- `unchecked_sub_assign` - i128 -= i128
- `unchecked_mul_assign` - u64 *= u64
- `unchecked_mul_div` - i128.mul_div()
- `unchecked_fixed_point_mul` - i128.fixed_point_mul()

**Safe patterns (MUST NOT flag):**
- `safe_add` - .checked_add()
- `safe_add_saturating` - .saturating_add()
- `safe_sub` - .checked_sub()
- `safe_sub_saturating` - .saturating_sub()
- `safe_mul` - .checked_mul()
- `safe_mul_saturating` - .saturating_mul()
- `safe_index_arithmetic` - buf[i + 1]

## CI Integration

The S003 rule runs automatically in CI pipelines:

1. **Contract CI** (`.github/workflows/contracts-ci.yml`) - Validates fixture contracts
2. **Rust CI** (`.github/workflows/rust.yml`) - Runs sanctifier-core tests
3. **SARIF Output** - Findings exported to SARIF format for IDE integration

## Known Limitations

### 1. Constant Expressions

The rule currently flags all arithmetic, including compile-time constants:

```rust
const TOTAL: u64 = 100 + 200;  // Currently flagged, but safe
```

**Future Enhancement**: Skip operations where both operands are compile-time constants.

### 2. Type-Level Guarantees

The rule doesn't understand type-level constraints:

```rust
fn add_small_numbers(a: u8, b: u8) -> u16 {
    (a as u16) + (b as u16)  // Safe due to widening, but flagged
}
```

**Workaround**: Cast to wider type before arithmetic, or suppress with `#[allow(unused)]` on checked version.

### 3. Cross-Function Analysis

The rule operates at function scope and doesn't track value ranges across function boundaries:

```rust
fn get_bounded_value() -> u64 {
    42  // Always returns 42
}

fn safe_add() -> u64 {
    get_bounded_value() + 1  // Flagged, but actually safe
}
```

**Mitigation**: Use checked operations defensively for contract safety.

## Configuration

### Enabling/Disabling

In `.sanctify.toml`:

```toml
enabled_rules = [
    "auth_gaps",
    "panics",
    "arithmetic",  # S003
    "ledger_size",
]
```

To disable:
```toml
enabled_rules = [
    "auth_gaps",
    "panics",
    # "arithmetic",  # Disabled
    "ledger_size",
]
```

### Severity

The default severity is `Medium`. To customize:

```rust
// In finding_codes.rs
FindingCode {
    code: ARITHMETIC_OVERFLOW,
    severity: FindingSeverity::High,  // Elevate to High
    // ...
}
```

## Output Examples

### CLI Output (Text)

```
⚠️  Unchecked Arithmetic
   → [S003] src/token.rs:transfer:42 — operator +
   → [S003] src/token.rs:transfer:45 — operator -
   → [S003] src/token.rs:mint:58 — operator *

  Suggestions:
  - Use .checked_add(rhs) or .saturating_add(rhs) to handle overflow
  - Use .checked_sub(rhs) or .saturating_sub(rhs) to handle underflow
  - Use .checked_mul(rhs) or .saturating_mul(rhs) to handle overflow
```

### JSON Output

```json
{
  "arithmetic_issues": [
    {
      "function_name": "transfer",
      "operation": "+",
      "suggestion": "Use .checked_add(rhs) or .saturating_add(rhs) to handle overflow",
      "location": "transfer:42"
    }
  ]
}
```

### SARIF Output

```json
{
  "results": [
    {
      "ruleId": "S003",
      "level": "warning",
      "message": {
        "text": "Unchecked '+' operation could overflow"
      },
      "locations": [{
        "physicalLocation": {
          "artifactLocation": { "uri": "src/token.rs" },
          "region": { "startLine": 42 }
        }
      }]
    }
  ]
}
```

## Related Findings

- **S002 (Panic Usage)**: Checked operations use `.expect()` which can panic
- **S016 (Truncation/Bounds)**: Integer casts and array indexing  
- **S026 (Taint Propagation)**: User-controlled arithmetic operands

## References

- [Rust Integer Overflow Behavior](https://doc.rust-lang.org/book/ch03-02-data-types.html#integer-overflow)
- [Soroban SDK Numeric Types](https://docs.rs/soroban-sdk/latest/soroban_sdk/)
- [CWE-190: Integer Overflow](https://cwe.mitre.org/data/definitions/190.html)
- [CWE-191: Integer Underflow](https://cwe.mitre.org/data/definitions/191.html)

## Changelog

### Version History

- **v0.1.0** (Initial) - Basic arithmetic operator detection
- **v0.2.0** - Added compound assignment operators (+=, -=, *=)
- **v0.3.0** - Added custom math methods (mul_div, fixed_point_*)
- **v0.4.0** - Excluded test code and index expressions
- **v0.5.0** - Added deduplication per (function, operator) pair
- **Current** - Production-ready with comprehensive test coverage

## Contribution Notes

### Adding New Patterns

To detect new arithmetic patterns:

1. Update `ArithVisitor::classify_op()` in `arithmetic_overflow.rs`
2. Add corresponding test case
3. Update this documentation
4. Add fixture example to `s003_arithmetic.rs`

### Modifying Behavior

When changing detection logic:

1. Update unit tests to reflect new behavior
2. Regenerate SARIF snapshots: `cargo insta review`
3. Update integration tests in `lib.rs`
4. Document the change in this file's Changelog section
5. Consider schema versioning if JSON output format changes

### Performance Considerations

The rule uses:
- **O(n)** AST traversal (single pass per file)
- **O(k)** deduplication where k = unique (function, operator) pairs
- **Memory**: HashSet for seen pairs (~8 bytes per pair)

Typical overhead: <5ms per 1000 lines of contract code.

## Support

For questions or issues:
- Open an issue: https://github.com/HyperSafeD/Sanctifier/issues
- Discussion: https://github.com/HyperSafeD/Sanctifier/discussions
- Security issues: security@sanctifier.dev

---

**Document Version:** 1.0.0  
**Last Updated:** 2026-02-25  
**Status:** Production Ready
