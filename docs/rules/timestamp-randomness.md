# Timestamp Randomness Rule (S029)

## Overview

The `timestamp_randomness` rule detects use of `env.ledger().timestamp()` as a source of randomness entropy. Block timestamps are **not** secret and can be nudged by validators within a small window, making any randomness derived solely from them manipulable.

## Severity

**High** — exploitable on-chain; a validator or a well-timed transaction can influence the outcome.

## Detection Logic

The rule fires when **all** of the following hold:

1. `env.ledger().timestamp()` is called inside a function or expression.
2. The context is randomness-related, identified by **name**:
   - Function name contains `rand`, `seed`, `pick`, or `winner` (case-insensitive), **OR**
   - A variable binding on the left-hand side of an assignment contains one of those keywords.

Non-sensitive uses of `env.ledger().timestamp()` (deadline checks, expiry guards, audit logs) are **not** flagged.

## Examples

### ❌ Vulnerable — function named `pick_winner`

```rust
pub fn pick_winner(env: Env, participants: Vec<Address>) -> Address {
    // UNSAFE: timestamp is predictable entropy
    let idx = env.ledger().timestamp() % participants.len() as u64;
    participants.get(idx as u32).unwrap()
}
```

### ❌ Vulnerable — variable named `seed`

```rust
pub fn initialize_game(env: Env) {
    // UNSAFE: 'seed' bound to timestamp
    let seed = env.ledger().timestamp();
    env.storage().persistent().set(&symbol_short!("seed"), &seed);
}
```

### ❌ Vulnerable — variable named `rand`

```rust
pub fn roll_dice(env: Env) -> u64 {
    let rand = env.ledger().timestamp() % 6 + 1;
    rand
}
```

### ✅ Safe — timestamp used for expiry only

```rust
pub fn check_expiry(env: Env, deadline: u64) -> bool {
    // SAFE: not randomness — pure time comparison
    env.ledger().timestamp() > deadline
}
```

### ✅ Safe — timestamp for audit logging

```rust
pub fn record_action(env: Env) {
    let ts = env.ledger().timestamp();
    env.storage().persistent().set(&symbol_short!("LAST_TS"), &ts);
}
```

## Why Timestamps Are Dangerous as Randomness

Soroban ledger timestamps represent the UNIX timestamp of the ledger close. Validators have limited but real influence over this value:

- A validator can delay or advance a ledger close within protocol bounds.
- An attacker who can predict or influence the timestamp can reverse-engineer the outcome of any computation based solely on it.
- For lottery/NFT draw/reward distribution use cases this translates to direct financial manipulation.

## Mitigation

### 1. Use a VRF Oracle (Recommended for High-Stakes)

```rust
pub fn pick_winner(
    env: Env,
    participants: Vec<Address>,
    vrf_proof: BytesN<64>,
) -> Address {
    // Verify the VRF proof, then derive index from it
    let idx = derive_index_from_vrf(&vrf_proof, participants.len() as u64);
    participants.get(idx as u32).unwrap()
}
```

### 2. Combine Multiple Entropy Sources

```rust
pub fn pick_winner(env: Env, participants: Vec<Address>) -> Address {
    let mut prng = env.prng();
    // Combine timestamp with ledger sequence and contract address hash
    let entropy = env.ledger().timestamp()
        ^ env.ledger().sequence() as u64
        ^ env.current_contract_address().to_string().len() as u64;
    prng.reseed(entropy);
    let idx = prng.gen_range(0..participants.len() as u64);
    participants.get(idx as u32).unwrap()
}
```

### 3. Commit-Reveal Scheme

Have participants commit a secret hash off-chain; reveal it on-chain before the draw. The XOR of all revealed secrets becomes the seed. This eliminates any single party's ability to manipulate the outcome.

## Related Rules

- **S018 — Unsafe PRNG** (`unsafe_prng`): flags `env.prng()` used in state-critical code without reseeding. See [unsafe-prng.md](unsafe-prng.md).
- **S006 — Unsafe Pattern** (`UNSAFE_PATTERN`): generic unsafe runtime patterns.

## References

- [Soroban Ledger API](https://docs.rs/soroban-sdk/latest/soroban_sdk/ledger/struct.Ledger.html)
- [OWASP: Insufficient Randomness](https://owasp.org/www-community/vulnerabilities/Insecure_Randomness)
- [CWE-338: Use of Cryptographically Weak PRNG](https://cwe.mitre.org/data/definitions/338.html)
- [SWC-120: Weak Sources of Randomness](https://swcregistry.io/docs/SWC-120)

## Testing

```bash
# Run the rule unit tests
cargo test -p sanctifier-core timestamp_randomness

# Test against the fixture contract
cargo test -p sanctifier-core -- timestamp_randomness
```

## Configuration

The rule is enabled by default. To disable it selectively in `.sanctify.toml`:

```toml
[rules]
timestamp_randomness = "off"
```
