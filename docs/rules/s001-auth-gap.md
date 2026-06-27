# S001 — Missing Authorization Guard (`auth_gap`)

**Category:** authentication  
**Severity:** Warning (Critical in `FindingCode` catalogue)  
**Rule name:** `auth_gap`

---

## What it detects

Any `pub` function inside an `impl` block that performs a **privileged operation** without
first calling `require_auth()` or `require_auth_for_args()`.  Privileged operations are:

| Operation class | Detected patterns |
|---|---|
| Storage mutation | `.set()`, `.update()`, `.remove()`, `.extend_ttl()` on `storage()`, `persistent()`, `temporary()`, or `instance()` |
| External contract call | `.invoke_contract()`, or a method call on a receiver whose type name ends in `Client` / `_client` |

Reserved Soroban entry-points (`__constructor`, `__check_auth`) are excluded automatically.  
Private (`fn`) and non-`impl` functions are not checked.

---

## Why it matters

A public function that writes state or calls external contracts without checking the caller's
identity can be invoked by **anyone** — draining balances, replacing the admin, or triggering
arbitrary cross-contract logic.  This is one of the most common critical findings in smart-contract
audits on Stellar.

---

## Examples

### Vulnerable — flagged by S001

```rust
impl Token {
    pub fn set_admin(env: Env, new_admin: Address) {
        // Missing require_auth — anyone can replace the admin.
        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }
}
```

### Fixed

```rust
impl Token {
    pub fn set_admin(env: Env, new_admin: Address) {
        new_admin.require_auth();                                    // ← guard added
        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }
}
```

### Also fixed — using `require_auth_for_args`

```rust
impl Token {
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth_for_args((&from, &to, amount).into_val(&env));
        // ... storage mutations ...
    }
}
```

---

## Remediation

1. Call `addr.require_auth()` (or `addr.require_auth_for_args(...)` — see [S030](require-auth-for-args.md))
   as the **first statement** in every state-mutating public function.
2. Consider whether read-only functions that expose sensitive data also need auth.
3. After adding the guard, run `sanctifier fix --rule auth_gap` to apply the automatic patch.

---

## Input validation

The rule validates its input before parsing:

| Input condition | Behaviour |
|---|---|
| Empty source | Returns no violations (no contract to analyse) |
| Source > 10 MB | Returns one `Error`-severity violation with code `SOURCE_TOO_LARGE` |
| Source contains null bytes | Returns one `Error`-severity violation with code `NULL_BYTE_DETECTED` |
| Source does not parse as valid Rust | Returns no violations (parse errors are surfaced separately by the CLI) |

---

## Auto-fix

`sanctifier fix` inserts `env.require_auth();` as the first statement in each flagged function.
Review the patch before committing — the correct auth target (caller vs. a specific `Address`
argument) depends on the function's semantics.

---

## Suppression

```rust
// sanctifier:ignore auth_gap
pub fn set_admin(env: Env, new_admin: Address) { ... }
```

Only suppress when you have a documented reason (e.g. the function is already called through an
authenticated wrapper contract).

---

## References

- Stellar SEP-41 — required auth patterns for token interfaces
- [S030 — Missing `require_auth_for_args`](require-auth-for-args.md)
- [Soroban auth model](https://soroban.stellar.org/docs/fundamentals-and-concepts/authorization)
- [docs/rule-authoring-guide.md](../rule-authoring-guide.md)
