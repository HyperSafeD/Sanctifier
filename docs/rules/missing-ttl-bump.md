# Missing TTL Bump (S025)

## Overview

The `missing_ttl_bump` rule detects writes to `persistent()` or `temporary()` Soroban storage tiers without a corresponding `extend_ttl` call in the same function. Entries in these tiers expire; without a TTL bump the data silently vanishes after the ledger TTL elapses.

## Severity

**Warning** — data loss will occur silently; fix before production deployment.

## Description

Soroban storage has three tiers with different lifetime semantics:

| Tier | Expires? | Needs extend_ttl? |
|------|----------|--------------------|
| `instance()` | Yes, but tied to contract instance | Managed separately |
| `persistent()` | Yes — after `max_ttl` ledgers | **Yes — bump each write** |
| `temporary()` | Yes — after a short TTL | **Yes — bump each write** |

When a function writes to `persistent()` or `temporary()` but never calls `extend_ttl`, the stored value may expire before it is ever read, causing silent data loss that is hard to debug.

## Examples

### ❌ Vulnerable Code

```rust
pub fn store(env: Env, key: Symbol, val: i128) {
    // Write without TTL bump — entry expires silently
    env.storage().persistent().set(&key, &val);
}
```

### ✅ Safe Code

```rust
pub fn store_safe(env: Env, key: Symbol, val: i128) {
    env.storage().persistent().set(&key, &val);
    // Bump TTL so the entry survives until the contract is next active
    env.storage().persistent().extend_ttl(&key, 1000, 5000);
}
```

## Mitigation

After every `persistent()` or `temporary()` write, call `extend_ttl` with appropriate `low` and `high` ledger thresholds.

```rust
env.storage().persistent().set(&key, &value);
env.storage().persistent().extend_ttl(&key, low_ttl, high_ttl);
```

For `instance()` storage, manage TTL at the contract-instance level via `env.storage().instance().extend_ttl(low, high)`.

## Related Rules

- **S004 `ledger_size`** — entry size approaching ledger limits
- **S021 `instance_storage_misuse`** — per-user data in Instance instead of Persistent

## References

- [Soroban Storage TTL](https://developers.stellar.org/docs/build/smart-contracts/storage/state-expiration)
- [extend_ttl API](https://docs.rs/soroban-sdk/latest/soroban_sdk/storage/struct.Storage.html)

## Testing

```bash
cargo test -p sanctifier-core missing_ttl_bump
cargo run -p sanctifier-cli -- analyze contracts/fixtures/finding-codes/s025_missing_ttl_bump.rs
```
