# Unsafe PRNG Example Contract

This contract demonstrates unsafe PRNG (Pseudo-Random Number Generator) usage patterns that the `unsafe_prng` rule detects.

## Vulnerability

Using `env.prng()` without proper seeding in state-critical code can lead to predictable randomness, which is a serious security vulnerability for:

- Lottery and gaming contracts
- Random token distribution
- NFT minting with random traits
- Any randomness-dependent logic that affects contract state

## Examples

### ❌ Unsafe Pattern (Will be flagged)

```rust
pub fn draw_winner_unsafe(env: Env, participants: Vec<Address>) -> Address {
    let random_index: u64 = env.prng().gen_range(0..participants.len() as u64);
    let winner = participants.get(random_index as u32).unwrap();

    // State mutation with predictable randomness
    env.storage()
        .persistent()
        .set(&symbol_short!("winner"), &winner);

    winner
}
```

### ✅ Safe Pattern (Will NOT be flagged)

```rust
pub fn get_random_number(env: Env) -> u64 {
    // Read-only operation - no state mutation
    env.prng().gen_range(0..100)
}
```

## Mitigation

For state-critical operations requiring randomness:

1. **Use external oracles** for truly unpredictable randomness
2. **Combine multiple entropy sources** (ledger timestamp, transaction hash, etc.)
3. **Document assumptions** if default seeding is sufficient for your use case
4. **Consider VRF** (Verifiable Random Function) for high-stakes applications

## Testing

Run the example contract tests:

```bash
cargo test -p unsafe-prng-example
```

Run the static analysis rule:

```bash
cargo test -p sanctifier-core unsafe_prng
```
