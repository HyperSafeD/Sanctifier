# Deposit-Withdraw — Auth-Gap Fixture (S001)

> **Sanctifier rule:** S001 — Missing `require_auth`  
> **Difficulty:** Easy · **Type:** Feature / Security fixture

---

## Purpose

This contract provides positive and negative fixtures for Sanctifier's **S001** rule,
which detects functions that modify state or transfer funds without first calling
`require_auth()` on the authorising address.

---

## Functions

| Function            | Auth? | Behaviour                                                |
|---------------------|-------|----------------------------------------------------------|
| `deposit`           | ✅    | Transfers tokens from caller into the vault.             |
| `withdraw_unsafe`   | ❌    | **Intentionally missing** `require_auth` — auth gap.     |
| `withdraw_safe`     | ✅    | Correct: calls `account.require_auth()` before transfer. |
| `balance`           | —     | Read-only: returns stored balance for an address.        |

---

## Attack Scenario — `withdraw_unsafe`

```
1. Victim calls deposit(victim, TOKEN, 1_000)
   → vault records Balance(victim) = 1_000

2. Attacker calls withdraw_unsafe(victim, TOKEN, 1_000)
   → contract skips auth check
   → Balance(victim) set to 0
   → 1_000 tokens transferred to env.invoker() (the attacker)

Result: victim loses all deposited funds with no signature required.
```

This is a classic **flash-loan amplified drain**:
- Borrow large amount → deposit on victim's behalf (if `deposit` were also unsafe) → drain → repay.
- Even without a flash loan, any on-chain observer can front-run a pending deposit.

---

## The Fix — One Line

```rust
// ❌ Before (auth gap)
pub fn withdraw_unsafe(env: Env, account: Address, token: Address, amount: i128) {
    // No auth check here
    ...
}

// ✅ After (secure)
pub fn withdraw_safe(env: Env, account: Address, token: Address, amount: i128) {
    account.require_auth();   // ← one line closes the gap
    ...
}
```

---

## Running

```bash
# From repo root
cd contracts/deposit-withdraw

# Run unit tests
cargo test

# Build WASM
cargo build --target wasm32-unknown-unknown --release

# Run Sanctifier scan (from repo root)
npx sanctifier scan contracts/deposit-withdraw/src/lib.rs
```

Expected Sanctifier output:

```
[S001] FAIL  contracts/deposit-withdraw/src/lib.rs:72  withdraw_unsafe — missing require_auth
[S001] PASS  contracts/deposit-withdraw/src/lib.rs:96  withdraw_safe   — require_auth present
```

---

## References

- [Sanctifier S001 rule docs](../../docs/rules/S001-missing-auth.md)
- [Soroban auth model](https://developers.stellar.org/docs/learn/smart-contract-internals/authorization)
