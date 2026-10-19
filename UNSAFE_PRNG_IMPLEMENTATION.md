# Unsafe PRNG Detection Rule Implementation

## Summary

Successfully implemented a new static analysis rule to detect unsafe PRNG (Pseudo-Random Number Generator) usage in Soroban smart contracts. The rule identifies cases where `env.prng()` is used in state-critical code without proper seeding, which can lead to predictable randomness vulnerabilities.

## What Was Implemented

### 1. Core Rule Implementation (`tooling/sanctifier-core/src/rules/unsafe_prng.rs`)

- **Rule Name**: `unsafe_prng`
- **Finding Code**: `S017`
- **Severity**: Warning
- **Detection Logic**:
  - Identifies functions using `env.prng()` or PRNG-related methods
  - Checks for storage mutations (`set`, `update`, `remove`, `extend_ttl`, `bump`)
  - Flags cases where PRNG is used with storage mutations but without `reseed()` calls

### 2. Finding Code Registration (`tooling/sanctifier-core/src/finding_codes.rs`)

- Added `UNSAFE_PRNG` constant with code `S017`
- Category: `randomness`
- Description: "Use of PRNG without proper seeding in state-critical code that could lead to predictable randomness"

### 3. Rule Registry Integration (`tooling/sanctifier-core/src/rules/mod.rs`)

- Registered `unsafe_prng` module
- Added rule to default rule registry
- Integrated with existing rule infrastructure

### 4. Example Contract (`contracts/unsafe-prng-example/`)

Demonstrates both vulnerable and safe patterns:

**Vulnerable Patterns (Flagged)**:

- `draw_winner_unsafe()` - Lottery without reseeding
- `distribute_rewards_unsafe()` - Token distribution with predictable randomness

**Safe Patterns (Not Flagged)**:

- `get_random_number()` - Read-only PRNG usage
- `set_value()` - Storage mutation without PRNG

### 5. Comprehensive Documentation

- **Rule Documentation** (`docs/rules/unsafe-prng.md`):
  - Detailed explanation of the vulnerability
  - Code examples (vulnerable and safe)
  - Mitigation strategies
  - References to security standards (CWE-338, OWASP)

- **Example Contract README** (`contracts/unsafe-prng-example/README.md`):
  - Quick reference for developers
  - Testing instructions

## Test Coverage

All tests passing (136 tests total):

```
✅ flags_prng_usage_with_storage_mutation_without_reseed
✅ no_violation_when_prng_reseeded
✅ no_violation_when_prng_without_storage_mutation
✅ no_violation_when_only_storage_mutation
✅ flags_v21_prng_host_functions
✅ empty_source_produces_no_findings
✅ invalid_source_produces_no_panic
```

## Security Impact

This rule helps prevent:

1. **Predictable Lottery Outcomes**: Attackers can't predict winners in lottery/gaming contracts
2. **Manipulated Token Distribution**: Random airdrops and rewards can't be gamed
3. **NFT Trait Manipulation**: Random trait generation becomes more secure
4. **General Randomness Vulnerabilities**: Any state-critical randomness is flagged

## Usage

### Running the Rule

```bash
# Test the rule
cargo test -p sanctifier-core unsafe_prng

# Test example contract
cargo test -p unsafe-prng-example

# Run all tests
cargo test -p sanctifier-core --lib
```

### Integration

The rule is automatically included in the default rule registry:

```rust
let registry = RuleRegistry::with_default_rules();
let violations = registry.run_all(source_code);
```

## Technical Details

### Detection Algorithm

1. Parse Rust source code using `syn` crate
2. Traverse AST to find function implementations
3. For each function:
   - Track PRNG usage (`env.prng()`, `gen_range()`, etc.)
   - Track storage mutations (`set()`, `update()`, etc.)
   - Track reseed calls (`reseed()`)
4. Flag if: `has_prng_usage && has_storage_mutation && !has_prng_reseed`

### Supported PRNG Methods

- `env.prng()` - Main PRNG accessor
- `gen_range()` - Generate random number in range
- `shuffle()` - Shuffle vector (v21)
- `prng_u64_in_inclusive_range()` - v21 host function
- `prng_bytes_new()` - v21 host function
- `prng_vec_shuffle()` - v21 host function

### Storage Mutation Detection

Detects mutations on:

- `env.storage().persistent()`
- `env.storage().temporary()`
- `env.storage().instance()`

Methods: `set`, `update`, `remove`, `extend_ttl`, `bump`

## Limitations and Future Work

### Current Limitations

1. **No Inter-Function Analysis**: Doesn't track PRNG usage across function calls
2. **Reseed Detection**: Only detects direct `reseed()` calls in the same function
3. **False Positives**: May flag non-critical randomness (cosmetic features)

### Future Enhancements

1. **Control Flow Analysis**: Track PRNG state across function boundaries
2. **Entropy Source Analysis**: Evaluate quality of reseeding entropy
3. **Configuration Options**: Allow users to specify acceptable patterns
4. **Auto-Fix Suggestions**: Propose specific reseeding strategies

## Related Work

- **S001 (AUTH_GAP)**: Missing authentication checks
- **S013 (REENTRANCY)**: Reentrancy vulnerabilities
- **S016 (TRUNCATION_BOUNDS)**: Integer truncation issues

## References

- [Soroban PRNG Documentation](https://docs.rs/soroban-sdk/latest/soroban_sdk/prng/struct.Prng.html)
- [CWE-338: Use of Cryptographically Weak PRNG](https://cwe.mitre.org/data/definitions/338.html)
- [OWASP: Insufficient Randomness](https://owasp.org/www-community/vulnerabilities/Insecure_Randomness)

## Conclusion

The unsafe PRNG detection rule successfully identifies a critical security vulnerability in Soroban smart contracts. It provides clear warnings with actionable suggestions, helping developers build more secure decentralized applications.
