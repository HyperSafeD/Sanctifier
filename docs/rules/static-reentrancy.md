# Static Reentrancy — External Call Before State Write (S027)

## Overview

The `static_reentrancy` rule detects the checks-effects-interactions (CEI) anti-pattern at the AST level: an external contract call (`invoke_contract` / `try_invoke_contract` / `invoke_contract_check`) that is followed by a storage mutation in the same function, without a reentrancy guard.

This rule is the **static complement** to the runtime reentrancy guard. It catches the pattern *before* deployment by inspecting the source AST.

## Severity

**Warning** — with a per-finding **confidence score** (high / medium / low).

## Description

The safe order for contract operations is:

1. **Checks** — validate inputs, read state
2. **Effects** — update storage state
3. **Interactions** — call external contracts

When interactions happen before effects, a malicious callee can re-enter the function and observe stale state, typically enabling double-spend or state corruption.

### Confidence Scoring

| Call type | Confidence | Reasoning |
|-----------|-----------|-----------|
| `invoke_contract` | **high** | Panics on failure; state write unreachable on error, so re-entrancy window is open on success |
| `try_invoke_contract` | **medium** | Recoverable, but state write still follows the call |
| `invoke_contract_check` | **medium** | Auth-checked call; re-entrancy still possible |
| Call inside a branch, write outside | **low** | Less direct, but still a potential vector |

## Examples

### ❌ Vulnerable Code (high confidence)

```rust
pub fn unsafe_withdraw(env: Env, other: Address, amount: i128) {
    // Interaction first — callee can re-enter and see old balance
    let _ = env.invoke_contract::<()>(&other, &symbol_short!("recv"), vec![&env]);
    // Effect too late
    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
}
```

### ✅ Safe Code — CEI order

```rust
pub fn safe_withdraw(env: Env, other: Address, amount: i128) {
    // Effects first
    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
    // Interaction last
    let _ = env.invoke_contract::<()>(&other, &symbol_short!("recv"), vec![&env]);
}
```

### ✅ Safe Code — reentrancy guard

```rust
pub fn guarded_call(env: Env, other: Address) {
    let locked: bool = env.storage().instance().get(&symbol_short!("RE_LOCK")).unwrap_or(false);
    if locked { panic!("reentrant call"); }
    env.storage().instance().set(&symbol_short!("RE_LOCK"), &true);
    let _ = env.invoke_contract::<()>(&other, &symbol_short!("cb"), vec![&env]);
    env.storage().persistent().set(&symbol_short!("STATE"), &1u32);
    env.storage().instance().set(&symbol_short!("RE_LOCK"), &false);
}
```

## Relationship to S013 (reentrancy)

| Rule | Pattern detected |
|------|-----------------|
| S013 `reentrancy` | State write **before** external call (mutation precedes call) |
| S027 `static_reentrancy` | External call **before** state write (call precedes mutation) |

Both patterns are dangerous; S027 catches the classic re-entrancy vector (stale-state exploitation on re-entry).

## False-Positive Corpus

See `benchmarks/static_reentrancy_fp_corpus.md` for a documented set of known safe patterns and their reasoning.

## Related Rules

- **S013 `reentrancy`** — state mutation before external call
- **S019 `unchecked_external_call`** — unhandled Result from cross-contract call
- **S022 `raw_invoke_contract`** — panicking invoke_contract usage

## Testing

```bash
cargo test -p sanctifier-core static_reentrancy
cargo run -p sanctifier-cli -- analyze contracts/fixtures/finding-codes/s027_static_reentrancy.rs
```
