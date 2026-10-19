# S012: SEP-41 Token Interface Compliance

**Category:** `token_interface`  
**Severity:** Critical  
**Finding Code:** `S012`

## Overview

The S012 check verifies that token contracts implement the complete [SEP-41 (Stellar Token Standard)](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md) interface with exact function signatures and proper authorization patterns.

SEP-41 defines a standard interface for fungible tokens on Stellar, ensuring interoperability between tokens, wallets, decentralized exchanges, and other smart contracts. Deviations from this standard can cause integration failures, authorization bypasses, or type mismatches.

## What S012 Checks

### 1. Function Presence (All 10 Required)

**Core Transfer Functions:**
- `allowance(env: Env, from: Address, spender: Address) -> i128`
- `approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32)`
- `balance(env: Env, id: Address) -> i128`
- `transfer(env: Env, from: Address, to: MuxedAddress, amount: i128)`
- `transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128)`

**Burn Functions:**
- `burn(env: Env, from: Address, amount: i128)`
- `burn_from(env: Env, spender: Address, from: Address, amount: i128)`

**Metadata Functions:**
- `decimals(env: Env) -> u32`
- `name(env: Env) -> String`
- `symbol(env: Env) -> String`

### 2. Signature Matching

Every function must have:
- **Exact parameter types** (including order)
- **Exact return type**
- **Correct parameter names** (informational, but helps catch copy-paste errors)

### 3. Authorization Patterns

Functions that mutate state must authorize the correct caller:

| Function | Must Authorize | Parameter Index |
|----------|----------------|-----------------|
| `approve` | `from` | 1 |
| `transfer` | `from` | 1 |
| `transfer_from` | `spender` | 1 |
| `burn` | `from` | 1 |
| `burn_from` | `spender` | 1 |

Authorization is detected via:
- `from.require_auth()` - Standard authorization
- `from.require_auth_for_args(...)` - Authorization with specific arguments

## Issue Types

### MissingFunction

A required SEP-41 function is not present in the contract.

**Example:**

```rust
#[contractimpl]
impl Token {
    pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
        from.require_auth();
        // ...
    }
    pub fn balance(env: Env, id: Address) -> i128 { 0 }
    pub fn name(env: Env) -> String { String::from_str(&env, "MyToken") }
    // Missing: allowance, approve, transfer_from, burn, burn_from, decimals, symbol
}
```

**Finding:**
```
S012 [MissingFunction]: Missing SEP-41 function 'allowance'.
Expected: allowance(env: Env, from: Address, spender: Address) -> i128
```

**Remediation:**
Add all 10 required functions to achieve full SEP-41 compliance.

---

### SignatureMismatch

A function exists but its signature doesn't match the SEP-41 specification.

**Example 1: Wrong Parameter Type**

```rust
// ❌ WRONG: Using Address instead of MuxedAddress for recipient
pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
    from.require_auth();
    // ...
}
```

**Finding:**
```
S012 [SignatureMismatch]: Function 'transfer' does not match the exact SEP-41 signature.
Expected: transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) -> ()
Actual:   transfer(env: Env, from: Address, to: Address, amount: i128) -> ()
```

**Remediation:**
```rust
// ✅ CORRECT: Use MuxedAddress for the recipient
pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
    from.require_auth();
    // ...
}
```

**Example 2: Wrong Return Type**

```rust
// ❌ WRONG: Returning u64 instead of i128
pub fn balance(env: Env, id: Address) -> u64 {
    // ...
}
```

**Finding:**
```
S012 [SignatureMismatch]: Function 'balance' does not match the exact SEP-41 signature.
Expected: balance(env: Env, id: Address) -> i128
Actual:   balance(env: Env, id: Address) -> u64
```

**Remediation:**
```rust
// ✅ CORRECT: Return i128
pub fn balance(env: Env, id: Address) -> i128 {
    // ...
}
```

**Example 3: Missing Parameter**

```rust
// ❌ WRONG: Missing expiration_ledger parameter
pub fn approve(env: Env, from: Address, spender: Address, amount: i128) {
    from.require_auth();
    // ...
}
```

**Finding:**
```
S012 [SignatureMismatch]: Function 'approve' does not match the exact SEP-41 signature.
Expected: approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) -> ()
Actual:   approve(env: Env, from: Address, spender: Address, amount: i128) -> ()
```

**Remediation:**
```rust
// ✅ CORRECT: Include all 5 parameters
pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
    from.require_auth();
    env.storage().persistent().set(
        &DataKey::Allowance(from.clone(), spender.clone()),
        &AllowanceValue { amount, expiration_ledger }
    );
}
```

---

### AuthorizationMismatch

A function has the correct signature but doesn't authorize the correct parameter.

**Example 1: Missing Authorization**

```rust
// ❌ WRONG: No authorization check
pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
    env.storage().persistent().set(
        &DataKey::Allowance(from.clone(), spender.clone()),
        &AllowanceValue { amount, expiration_ledger }
    );
}
```

**Finding:**
```
S012 [AuthorizationMismatch]: Function 'approve' should authorize 'from' to match the SEP-41 interface.
Expected: approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) -> ()
```

**Remediation:**
```rust
// ✅ CORRECT: Authorize the 'from' parameter
pub fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
    from.require_auth();
    env.storage().persistent().set(
        &DataKey::Allowance(from.clone(), spender.clone()),
        &AllowanceValue { amount, expiration_ledger }
    );
}
```

**Example 2: Wrong Parameter Authorized**

```rust
// ❌ WRONG: Authorizing 'from' instead of 'spender'
pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
    from.require_auth(); // Wrong!
    // ...
}
```

**Finding:**
```
S012 [AuthorizationMismatch]: Function 'transfer_from' should authorize 'spender' to match the SEP-41 interface.
```

**Remediation:**
```rust
// ✅ CORRECT: Authorize the 'spender' parameter
pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
    spender.require_auth();
    
    // Decrement allowance
    let allowance = read_allowance(&env, from.clone(), spender.clone());
    if allowance.amount < amount {
        panic!("insufficient allowance");
    }
    spend_allowance(&env, from.clone(), spender.clone(), amount, &allowance);
    
    // Perform transfer
    transfer_impl(&env, from, to, amount);
}
```

## Candidate Detection

Not every contract is checked for SEP-41 compliance. The checker uses a heuristic to identify **token candidates**:

**A contract is considered a token candidate if it has:**
- **≥2 core functions** (allowance, approve, balance, transfer, transfer_from, burn, burn_from), OR
- **≥1 core function + ≥2 metadata functions** (decimals, name, symbol)

**Examples:**

```rust
// ✅ Candidate: 2 core functions
impl Token {
    pub fn transfer(...) {}
    pub fn balance(...) -> i128 { 0 }
}

// ✅ Candidate: 1 core + 2 metadata
impl Token {
    pub fn transfer(...) {}
    pub fn name(...) -> String {}
    pub fn symbol(...) -> String {}
}

// ❌ NOT a candidate: unrelated contract
impl Counter {
    pub fn increment(...) {}
    pub fn get(...) -> u32 { 0 }
}
```

**Why this heuristic?**
- **Too strict** (e.g., requiring all 10): Won't catch incomplete implementations during development
- **Too loose** (e.g., any 1 function): Floods output with false positives
- **Just right**: Catches real tokens while avoiding false alarms

## Special Cases

### MuxedAddress in `transfer`

The `transfer` function uses `MuxedAddress` for the recipient (parameter 3) instead of `Address`.

**Why?** `MuxedAddress` enables Stellar's multiplexed account feature, allowing a single account to have multiple sub-accounts without requiring memo fields. This is a deliberate SEP-41 design choice.

**In practice:** `MuxedAddress` is semantically equivalent to `Address` in most Soroban contracts, but using the wrong type will trigger a `SignatureMismatch` finding.

### Authorization Methods

Both authorization methods are accepted:
- `from.require_auth()` - Standard method
- `from.require_auth_for_args(vec![...])` - Authorization with specific arguments (for sub-authorizations)

### Private Functions

Only **public** functions are checked. Private helper functions are ignored:

```rust
#[contractimpl]
impl Token {
    pub fn transfer(...) { ... } // ✅ Checked
    
    fn internal_validate(...) { ... } // ⏭️ Ignored (private)
}
```

## Limitations

S012 validates **interface compliance** but does NOT verify:

1. **Allowance Decrements**: Whether `transfer_from` correctly decrements allowances → See S024
2. **Total Supply Invariants**: Whether token supply is correctly tracked → See S011 (formal verification)
3. **Reentrancy Protection**: Whether state mutations happen before external calls → See S015
4. **Arithmetic Overflow**: Handled automatically by Soroban's safe math runtime
5. **Authorization Timing**: Whether authorization happens before state changes (future enhancement)

## Reference Implementation

See [`contracts/my-contract/src/lib.rs`](../../contracts/my-contract/src/lib.rs) for a fully compliant SEP-41 token implementation.

## Testing

- **Unit Tests**: `tooling/sanctifier-core/src/sep41.rs` (5 tests)
- **Integration Tests**: `tooling/sanctifier-core/tests/sep41_tests.rs` (20+ tests covering all issue types)
- **Fixtures**: `contracts/fixtures/finding-codes/s012_token_interface.rs`
- **CI Validation**: `.github/workflows/ci.yml` verifies S012 appears in fixture analysis output

## Related Checks

- **S001 (auth_gap)**: Detects missing authorization in general contracts (S012 is token-specific)
- **S024 (transfer_from_no_allowance)**: Verifies `transfer_from` decrements allowances
- **S011 (smt_invariant)**: Formally verifies total supply invariants via Kani

## Configuration

S012 cannot be disabled as it's critical for token interoperability. To adjust behavior:

1. **Suppress for non-production tokens**: Add `#[allow(sanctifier::s012)]` attribute (future enhancement)
2. **Custom candidate detection**: Modify `looks_like_sep41_candidate()` in `sep41.rs`
3. **Relaxed authorization**: Not recommended - breaks SEP-41 compliance

## Contributing

When modifying S012 checks:

1. Update `SEP41_FUNCTIONS` constant if the spec changes
2. Add tests in both `sep41.rs` and `tests/sep41_tests.rs`
3. Update this documentation with examples
4. Ensure CI passes for the fixtures
5. Document any breaking changes in `CHANGELOG.md`

## Further Reading

- [SEP-41 Specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)
- [Soroban Token Interface](https://soroban.stellar.org/docs/reference/interfaces/token-interface)
- [Authorization in Soroban](https://soroban.stellar.org/docs/learn/authorization)
- [MuxedAddress Documentation](https://developers.stellar.org/docs/encyclopedia/muxed-accounts)
