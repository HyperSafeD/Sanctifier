# Unsafe PRNG Rule (S017)

## Overview

The `unsafe_prng` rule detects use of Soroban's PRNG (Pseudo-Random Number Generator) without proper seeding in state-critical code. This vulnerability can lead to predictable randomness, which is dangerous for security-sensitive operations.

## Severity

**Warning** - This is a security concern that should be addressed before production deployment.

## Description

Soroban provides a PRNG through `env.prng()` for generating random values. However, if not properly seeded with unpredictable entropy, the PRNG can produce predictable sequences. This is particularly dangerous when:

1. The random values influence contract state (storage mutations)
2. The randomness affects token distribution, lottery outcomes, or other value transfers
3. Security-critical decisions depend on the random values

## Detection Logic

The rule flags functions that:

1. ✅ Use `env.prng()` or PRNG-related methods (`gen_range`, `shuffle`, etc.)
2. ✅ Perform storage mutations (`set`, `update`, `remove`, `extend_ttl`, `bump`)
3. ❌ Do NOT call `reseed()` before using the PRNG

## Examples

### ❌ Vulnerable Code

```rust
pub fn draw_winner(env: Env, participants: Vec<Address>) -> Address {
    // UNSAFE: Using PRNG without reseeding
    let random_index: u64 = env.prng().gen_range(0..participants.len() as u64);
    let winner = participants.get(random_index as u32).unwrap();

    // State mutation makes this critical
    env.storage()
        .persistent()
        .set(&symbol_short!("winner"), &winner);

    winner
}
```

**Why this is dangerous:**

- Attackers can predict the random sequence
- Lottery outcomes can be manipulated
- Winners can be predetermined

### ❌ Another Vulnerable Pattern

```rust
pub fn distribute_rewards(env: Env, recipients: Vec<Address>, amount: i128) {
    // UNSAFE: Random bonus without proper seeding
    let random_bonus: u64 = env.prng().gen_range(1..10);

    for recipient in recipients.iter() {
        let final_amount = if random_bonus > 5 {
            amount * 2
        } else {
            amount
        };

        // State mutation based on predictable randomness
        env.storage()
            .persistent()
            .set(&recipient, &final_amount);
    }
}
```

### ✅ Safe Code (No Storage Mutation)

```rust
pub fn get_random_number(env: Env) -> u64 {
    // SAFE: Read-only operation, no state mutation
    env.prng().gen_range(0..100)
}
```

**Why this is safe:**

- No storage mutations
- Result doesn't affect contract state
- Predictability doesn't create security risk

### ✅ Safe Code (With Reseeding)

```rust
pub fn draw_winner_safe(env: Env, participants: Vec<Address>) -> Address {
    let mut prng = env.prng();

    // SAFE: Reseed with unpredictable entropy
    prng.reseed(env.ledger().timestamp());

    let random_index: u64 = prng.gen_range(0..participants.len() as u64);
    let winner = participants.get(random_index as u32).unwrap();

    env.storage()
        .persistent()
        .set(&symbol_short!("winner"), &winner);

    winner
}
```

**Why this is safer:**

- Reseeded with ledger timestamp (harder to predict)
- Combines multiple entropy sources
- Reduces predictability window

## Mitigation Strategies

### 1. Use External Oracles (Recommended for High-Stakes)

```rust
pub fn draw_winner_with_oracle(
    env: Env,
    participants: Vec<Address>,
    oracle_randomness: BytesN<32>
) -> Address {
    // Use externally provided randomness from VRF or oracle
    let random_index = u64::from_be_bytes(
        oracle_randomness.slice(0..8).try_into().unwrap()
    ) % participants.len() as u64;

    let winner = participants.get(random_index as u32).unwrap();
    env.storage().persistent().set(&symbol_short!("winner"), &winner);
    winner
}
```

### 2. Combine Multiple Entropy Sources

```rust
pub fn draw_winner_multi_entropy(env: Env, participants: Vec<Address>) -> Address {
    let mut prng = env.prng();

    // Combine multiple unpredictable sources
    let entropy = env.ledger().timestamp()
        ^ env.ledger().sequence()
        ^ env.current_contract_address().to_string().len() as u64;

    prng.reseed(entropy);

    let random_index: u64 = prng.gen_range(0..participants.len() as u64);
    let winner = participants.get(random_index as u32).unwrap();

    env.storage().persistent().set(&symbol_short!("winner"), &winner);
    winner
}
```

### 3. Document Assumptions

If default seeding is sufficient for your use case (e.g., non-financial randomness), document why:

```rust
/// Generates a random color for cosmetic purposes only.
/// Note: Uses default PRNG seeding as this is not security-critical.
/// The randomness is for user experience, not value distribution.
pub fn generate_random_color(env: Env) -> u32 {
    let color: u32 = env.prng().gen_range(0..0xFFFFFF);
    env.storage().persistent().set(&symbol_short!("color"), &color);
    color
}
```

## Limitations

The rule may produce false positives for:

- Non-critical randomness (cosmetic features)
- Test/development code
- Cases where predictability is acceptable

In these cases, document your reasoning and consider suppressing the warning.

## Related Vulnerabilities

- **Weak Randomness (CWE-338)**: Cryptographically weak PRNG
- **Predictable Seed (CWE-337)**: Use of predictable seed values
- **Insufficient Entropy (CWE-331)**: Insufficient randomness

## References

- [Soroban PRNG Documentation](https://docs.rs/soroban-sdk/latest/soroban_sdk/prng/struct.Prng.html)
- [OWASP: Insufficient Randomness](https://owasp.org/www-community/vulnerabilities/Insecure_Randomness)
- [CWE-338: Use of Cryptographically Weak PRNG](https://cwe.mitre.org/data/definitions/338.html)

## Testing

Test the rule against example contracts:

```bash
# Run rule tests
cargo test -p sanctifier-core unsafe_prng

# Test against example contract
cargo test -p unsafe-prng-example
```

## Configuration

This rule is enabled by default in the Sanctifier rule registry. To disable it:

```rust
let mut registry = RuleRegistry::new();
// Register all rules except unsafe_prng
registry.register(auth_gap::AuthGapRule::new());
// ... other rules
```
