# Static Reentrancy — False-Positive Corpus & Evaluation

This document catalogues known-safe patterns that the `static_reentrancy` (S027)
rule must **not** flag, along with the reasoning for each.

---

## Corpus Entries

### FP-01: Correct CEI order (write before call)

```rust
pub fn safe_withdraw(env: Env, other: Address, amount: i128) {
    // Effects first
    env.storage().persistent().set(&symbol_short!("BAL"), &amount);
    // Interaction last — correct CEI
    let _ = env.invoke_contract::<()>(&other, &symbol_short!("recv"), vec![&env]);
}
```

**Expected result:** No violation.  
**Reason:** Storage mutation happens before the external call — the canonical safe order.

---

### FP-02: External call with no subsequent write

```rust
pub fn just_call(env: Env, other: Address) {
    let _ = env.invoke_contract::<()>(&other, &symbol_short!("ping"), vec![&env]);
}
```

**Expected result:** No violation.  
**Reason:** There is no storage mutation following the call; no reentrancy window exists.

---

### FP-03: Reentrancy guard present

```rust
pub fn guarded_call(env: Env, other: Address) {
    let locked: bool = env.storage().instance()
        .get(&symbol_short!("RE_LOCK")).unwrap_or(false);
    if locked { panic!("reentrant call"); }
    env.storage().instance().set(&symbol_short!("RE_LOCK"), &true);
    let _ = env.invoke_contract::<()>(&other, &symbol_short!("cb"), vec![&env]);
    env.storage().persistent().set(&symbol_short!("STATE"), &1u32);
    env.storage().instance().set(&symbol_short!("RE_LOCK"), &false);
}
```

**Expected result:** No violation.  
**Reason:** The `REENTRANCY_LOCK` / `RE_LOCK` guard pattern is detected; re-entry will panic.

---

### FP-04: Read-only function

```rust
pub fn query(env: Env) -> i128 {
    env.storage().persistent().get(&symbol_short!("BAL")).unwrap_or(0)
}
```

**Expected result:** No violation.  
**Reason:** No storage mutations; no call → write sequence possible.

---

### FP-05: Multiple writes all before a single call

```rust
pub fn multi_write_then_call(env: Env, other: Address) {
    env.storage().persistent().set(&symbol_short!("A"), &1u32);
    env.storage().persistent().set(&symbol_short!("B"), &2u32);
    let _ = env.invoke_contract::<()>(&other, &symbol_short!("go"), vec![&env]);
}
```

**Expected result:** No violation.  
**Reason:** All writes precede the external call; CEI order is satisfied.

---

## Evaluation Summary

| ID | Pattern | Rule result | Expected | Pass? |
|----|---------|-------------|----------|-------|
| FP-01 | CEI correct order | No violation | No violation | ✅ |
| FP-02 | Call without write | No violation | No violation | ✅ |
| FP-03 | Guard present | No violation | No violation | ✅ |
| FP-04 | Read-only | No violation | No violation | ✅ |
| FP-05 | All writes before call | No violation | No violation | ✅ |

---

## Known True Positives (for completeness)

| Pattern | Confidence |
|---------|-----------|
| `invoke_contract` → `persistent().set()` | high |
| `try_invoke_contract` → `persistent().set()` | medium |
| `invoke_contract_check` → `instance().set()` | medium |
| Call in `if` branch → write outside | low |
