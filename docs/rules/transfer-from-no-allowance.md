# Transfer-From Without Allowance Check (S023)

## Overview

The `transfer_from_no_allowance` rule detects `transfer_from`-style functions that consume a `from` account's balance without first checking or decrementing the spender's allowance. This vulnerability allows **any caller to drain any account** without permission.

## Severity

**Error** — This is a critical security vulnerability that must be fixed before deployment.

## Description

Token contracts commonly implement a delegated transfer pattern (`transfer_from`) where a `spender` is allowed to move tokens on behalf of a `from` account, up to an approved allowance. When the allowance check is missing, there is no gate on who can call the function or how much they can move, breaking the fundamental token accounting model.

The rule flags public functions that:

1. ✅ Are named `transfer_from` / `transferFrom` (or match the spender+from+to signature pattern)
2. ✅ Perform storage mutations on the `from` balance
3. ❌ Do NOT reference any allowance (no `allowance`, `spend_limit`, or `approved` in the body)

## Examples

### ❌ Vulnerable Code

```rust
// VULNERABILITY: Any caller can pass any 'from' address and drain their balance.
pub fn transfer_from(e: Env, _spender: Address, from: Address, to: Address, amount: i128) {
    let from_bal: i128 = e.storage().persistent().get(&from).unwrap_or(0);
    e.storage().persistent().set(&from, &(from_bal - amount));

    let to_bal: i128 = e.storage().persistent().get(&to).unwrap_or(0);
    e.storage().persistent().set(&to, &(to_bal + amount));
}
```

**Why this is dangerous:**

- No allowance check — any caller can transfer tokens from any account
- Combined with missing `require_auth`, this is a complete loss of funds vulnerability
- Directly violates the SEP-41 token standard

### ✅ Safe Code

```rust
pub fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
    spender.require_auth();

    // Check and decrement allowance before moving balance
    let allowance = get_allowance(&e, from.clone(), spender.clone());
    assert!(allowance >= amount, "insufficient allowance");
    set_allowance(&e, from.clone(), spender.clone(), allowance - amount);

    // Now safe to move the balance
    let from_bal: i128 = e.storage().persistent().get(&from).unwrap_or(0);
    e.storage().persistent().set(&from, &(from_bal - amount));

    let to_bal: i128 = e.storage().persistent().get(&to).unwrap_or(0);
    e.storage().persistent().set(&to, &(to_bal + amount));
}
```

**Why this is safe:**

- `spender.require_auth()` ensures the caller is who they claim to be
- Allowance is read and verified before any balance mutation
- Allowance is decremented atomically with the transfer

## Mitigation

1. Always read the spender's allowance from storage before touching `from`'s balance
2. Assert `allowance >= amount` and revert otherwise
3. Decrement the allowance in the same transaction as the balance change
4. Call `spender.require_auth()` at the top of the function

## Related Rules

- **S001 `auth_gap`** — public function mutates state without any `require_auth`
- **S012 `sep41_interface`** — SEP-41 token interface deviation
- **S013 `reentrancy`** — state mutation before external call without a reentrancy guard

## References

- [SEP-41 Token Interface](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)
- [Soroban Auth Documentation](https://developers.stellar.org/docs/build/smart-contracts/example-contracts/auth)
- [CWE-862: Missing Authorization](https://cwe.mitre.org/data/definitions/862.html)

## Testing

```bash
# Run rule unit tests
cargo test -p sanctifier-core transfer_from_no_allowance

# Verify the fixture contract is detected
cargo run -p sanctifier-cli -- analyze contracts/token-with-bugs/src/lib.rs
```
