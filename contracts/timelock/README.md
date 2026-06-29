# Timelock Controller Contract

A teaching-focused Soroban smart contract demonstrating timelock mechanisms with both **safe** and **vulnerable** implementations.

## Overview

The contract implements a role-based timelock system with:
- **Proposers**: Schedule calls with a minimum delay
- **Executors**: Execute scheduled calls after the delay elapses  
- **Cancellers**: Abort pending operations
- **Admin**: Manage roles and update the delay

## Key Implementations

### ✓ Safe: `execute()`

The safe implementation **enforces the time delay** before allowing execution.

```rust
pub fn execute(
    env: Env,
    executor: Address,
    target: Address,
    fn_name: Symbol,
    args: Vec<Val>,
    salt: BytesN<32>,
) -> Val {
    executor.require_auth();
    if !Self::is_executor(env.clone(), executor) {
        panic_with_error!(&env, TimelockError::Unauthorized);
    }

    let hash = compute_hash(&env, &target, &fn_name, &args, &salt);
    let ready_timestamp: u64 = env.storage().instance().get(&DataKey::Proposal(hash.clone()))
        .unwrap_or_else(|| panic_with_error!(&env, TimelockError::ProposalNotFound));

    // ✓ SAFETY CHECK: Verify delay has elapsed
    if env.ledger().timestamp() < ready_timestamp {
        panic_with_error!(&env, TimelockError::ProposalNotReady);
    }

    env.storage().instance().remove(&DataKey::Proposal(hash.clone()));
    env.invoke_contract(&target, &fn_name, args)
}
```

**Security Guarantee**: A scheduled operation cannot execute until `ready_timestamp` has passed. The delay check happens *before* any cross-contract call.

**Test**: [`test_safe_execute_enforces_delay`](src/test.rs) — verifies that execution fails if attempted before the delay elapses.

---

### ✗ Vulnerable: `execute_unsafe()`

The unsafe implementation **bypasses the delay check**, allowing immediate execution.

```rust
pub fn execute_unsafe(
    env: Env,
    executor: Address,
    target: Address,
    fn_name: Symbol,
    args: Vec<Val>,
    salt: BytesN<32>,
) -> Val {
    executor.require_auth();
    if !Self::is_executor(env.clone(), executor) {
        panic_with_error!(&env, TimelockError::Unauthorized);
    }

    let hash = compute_hash(&env, &target, &fn_name, &args, &salt);
    
    // ✗ VULNERABILITY: No delay check! Proposal can be executed immediately.
    let _ready_timestamp: u64 = env.storage().instance().get(&DataKey::ProposalUnsafe(hash.clone()))
        .unwrap_or_else(|| panic_with_error!(&env, TimelockError::ProposalNotFound));

    env.storage().instance().remove(&DataKey::ProposalUnsafe(hash.clone()));
    
    // ✗ Executes WITHOUT enforcing any delay
    env.invoke_contract(&target, &fn_name, args)
}
```

**The Bug**: The `ready_timestamp` is fetched from storage but never compared against `env.ledger().timestamp()`. An executor can bypass the timelock entirely.

**Impact**: 
- Urgent governance actions can be executed before the delay period
- Admin parameter changes (e.g., pause mechanisms, fee updates) are not protected
- Gives attackers a window to front-run or exploit mid-transaction state

**Test**: [`test_execute_unsafe_bypasses_delay`](src/test.rs) — demonstrates that execution succeeds immediately without waiting for any delay.

---

## Test Suite

All tests pass with `cargo test`:

```
test test::test_timelock_flow ... ok
test test::test_execute_unsafe_bypasses_delay ... ok
test test::test_safe_execute_enforces_delay ... ok
test test::test_role_management ... ok
test test::test_update_delay ... ok
test test::prop_delay_below_min_is_invalid ... ok
test test::prop_ready_exactly_at_delay ... ok
test test::prop_not_ready_before_delay ... ok
test test::prop_delay_overflow_is_never_ready ... ok
```

### Running Tests

```bash
cd contracts/timelock
cargo test
```

### Specific Test: Safe vs Unsafe

```bash
cargo test test_safe_execute_enforces_delay -- --nocapture
cargo test test_execute_unsafe_bypasses_delay -- --nocapture
```

---

## Sanctifier Analysis

### What Sanctifier Detects

Sanctifier will flag the missing delay enforcement in `execute_unsafe()` through multiple mechanisms:

1. **S006 (Unsafe Patterns)**: The missing check is an omitted validation pattern.
2. **Custom Rules** (S007): A rule detecting unprotected cross-contract invocations can catch this.
3. **Data Flow Analysis**: Sanctifier's taint engine can identify that `ready_timestamp` is loaded but never used.

### Running Analysis

```bash
sanctifier analyze ./contracts/timelock --format json > timelock-report.json
```

### Expected Findings

The analysis should flag:
- **Location**: `execute_unsafe()` at the `env.invoke_contract()` call
- **Category**: Missing validation / unsafe pattern
- **Message**: "Cross-contract call without delay validation; ready_timestamp loaded but not checked"
- **Severity**: High (can bypass critical security mechanism)

### Why This Matters

This contract serves as a **teaching example** for Sanctifier's ability to detect:
- ✗ Logic errors (omitted checks)
- ✗ Unused variables (loaded but not consumed)
- ✗ Insufficient authorization (correct auth role but wrong logic)
- ✓ Contrast with safe patterns

---

## How to Use

### For Security Researchers

- Study both implementations side-by-side
- Run the safe and unsafe tests
- Extend tests to explore edge cases (e.g., overflow in delay calculation)
- Run Sanctifier to see how it detects the vulnerability

### For Auditors

- Use this as a reference for timelock pitfalls
- Add it to your checklist: "Verify delay is checked *before* execution"
- Note the pattern: fetch → validate → execute

### For Developers

- Always implement delay checks **before** side effects
- Use property-based tests (proptest) to verify delay logic
- Consider using a wrapper macro to ensure consistency

---

## Implementation Notes

### Storage Schema

Two separate storage keys distinguish safe from unsafe proposals:
- `DataKey::Proposal(hash)` → safe proposals (with delay)
- `DataKey::ProposalUnsafe(hash)` → unsafe proposals (no delay)

This separation makes the vulnerability explicit and prevents accidental mixing.

### Delay Calculation

Delays are stored as an absolute timestamp (`proposal_time + delay`). This avoids:
- Off-by-one errors in block counting
- Ledger timestamp rollback scenarios
- Overflow if delay is huge (uses `checked_add`)

---

## References

- [Runtime Guards Integration](../runtime-guard-wrapper/README.md) — wrap this contract to log all executions
- [Soroban Timelock Pattern](https://github.com/stellar/soroban-examples) — OpenZeppelin-style reference
- [Sanctifier S006 Rule](../../docs/rules/s006-unsafe-patterns.md) — what patterns trigger this finding

---

## License

MIT
