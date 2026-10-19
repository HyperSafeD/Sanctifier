# require_auth_for_args Rule (S030)

## Overview

The `require_auth_for_args` rule detects functions with multiple `Address` parameters that use `require_auth()` instead of `require_auth_for_args()`. This vulnerability enables replay and scope-confusion attacks on multi-argument admin operations.

## Severity

**High** - This is a critical security vulnerability that can lead to unauthorized operations and privilege escalation.

## Description

In Soroban smart contracts, authentication is crucial for privileged operations. While `require_auth()` verifies that a caller has authorized the transaction, it does **not** bind the authorization to specific function arguments. This creates a vulnerability in functions with multiple `Address` parameters.

### The Problem

When a function has multiple `Address` parameters (e.g., `set_admin(caller: Address, new_admin: Address)`), using `require_auth()` only verifies that `caller` signed _something_, but not _what_ they signed. An attacker can:

1. **Replay Attack**: Capture a valid signature for one set of arguments and replay it with different arguments
2. **Scope Confusion**: Use a signature intended for one operation in a different context with different parameters

### Why It Matters

This vulnerability is particularly dangerous for:

- **Admin operations**: `set_admin(caller, new_admin)` - attacker can change admin to any address
- **Multi-party transfers**: `transfer_from(spender, from, to, amount)` - attacker can redirect funds
- **Permission grants**: `grant_role(admin, user, role)` - attacker can grant arbitrary roles
- **Delegation**: `set_proposer(admin, proposer)` - attacker can change proposers

## Detection Logic

The rule flags functions that meet **all** of the following criteria:

1. ✅ Function is `pub` (public)
2. ✅ Function has **2 or more** `Address` parameters (excluding `Env`)
3. ✅ Function performs sensitive operations:
   - Storage mutations (`.set()`, `.update()`, `.remove()`, `.extend_ttl()`, `.bump()`)
   - External contract calls (`.invoke_contract()`, `.try_invoke_contract()`, `.call()`)
4. ✅ Function uses `require_auth()`
5. ❌ Function does **NOT** use `require_auth_for_args()`

## Examples

### ❌ Vulnerable Code

```rust
#[contractimpl]
impl AdminContract {
    /// VULNERABLE: Attacker can replay signature with different new_admin
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth();  // ❌ Only verifies caller signed, not what they signed
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &new_admin);
    }

    /// VULNERABLE: Attacker can redirect transfer to different addresses
    pub fn transfer_from(
        env: Env,
        spender: Address,
        from: Address,
        to: Address,
        amount: i128
    ) {
        spender.require_auth();  // ❌ Doesn't bind to from/to/amount
        // ... transfer logic
    }
}
```

**Attack Scenario for `set_admin`:**

1. Alice (legitimate admin) calls `set_admin(alice, bob)` to make Bob the new admin
2. Alice's signature authorizes the transaction
3. Attacker intercepts the transaction
4. Attacker replays Alice's signature but changes `new_admin` to attacker's address
5. Contract accepts it because `alice.require_auth()` only checks Alice signed, not what she signed
6. Attacker is now admin

### ✅ Safe Code

```rust
#[contractimpl]
impl AdminContract {
    /// SAFE: Authorization bound to exact arguments
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) {
        // Bind authorization to the new_admin argument
        caller.require_auth_for_args(
            (new_admin.clone(),).into_val(&env)
        );
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &new_admin);
    }

    /// SAFE: Authorization bound to all critical parameters
    pub fn transfer_from(
        env: Env,
        spender: Address,
        from: Address,
        to: Address,
        amount: i128
    ) {
        // Bind authorization to from, to, and amount
        spender.require_auth_for_args(
            (from.clone(), to.clone(), amount).into_val(&env)
        );
        // ... transfer logic
    }
}
```

### ✅ Safe Code - Single Address Parameter

```rust
#[contractimpl]
impl SimpleContract {
    /// SAFE: Single Address parameter - require_auth is appropriate
    pub fn set_owner(env: Env, owner: Address) {
        owner.require_auth();  // ✅ OK for single-address functions
        env.storage()
            .instance()
            .set(&symbol_short!("owner"), &owner);
    }
}
```

**Why this is safe:** With only one `Address` parameter, there's no ambiguity about what's being authorized. The signature is inherently bound to the transaction context.

## Mitigation Strategies

### 1. Use `require_auth_for_args()` for Multi-Address Functions

Replace `require_auth()` with `require_auth_for_args()` and pass all critical parameters:

```rust
// Before (vulnerable)
caller.require_auth();

// After (safe)
caller.require_auth_for_args(
    (new_admin.clone(), role.clone()).into_val(&env)
);
```

### 2. Include All Security-Critical Parameters

Bind authorization to **all** parameters that affect the operation's security:

```rust
pub fn approve_transfer(
    env: Env,
    owner: Address,
    spender: Address,
    amount: i128,
    expiration: u64
) {
    // Include ALL critical parameters
    owner.require_auth_for_args(
        (spender.clone(), amount, expiration).into_val(&env)
    );
    // ... approval logic
}
```

### 3. Understand the `.into_val(&env)` Conversion

The arguments must be converted to Soroban's value representation:

```rust
// Single argument - use tuple with trailing comma
caller.require_auth_for_args((arg1.clone(),).into_val(&env));

// Multiple arguments - use tuple
caller.require_auth_for_args((arg1.clone(), arg2, arg3).into_val(&env));
```

## When `require_auth()` is Acceptable

Use `require_auth()` (without args) when:

1. **Single Address parameter**: No ambiguity about what's being authorized
2. **Read-only operations**: No state changes or external calls
3. **Private functions**: Not exposed as contract entry points

## Related Rules

- **S001 (AUTH_GAP)**: Detects missing authentication entirely
- **S024 (TRANSFER_FROM_NO_ALLOWANCE)**: Detects missing allowance checks in transfer_from
- **S013 (REENTRANCY)**: Detects reentrancy vulnerabilities

## References

- [Soroban Address Documentation](https://docs.rs/soroban-sdk/latest/soroban_sdk/struct.Address.html)
- [Soroban Authorization Guide](https://soroban.stellar.org/docs/learn/authorization)
- [require_auth_for_args API](https://docs.rs/soroban-sdk/latest/soroban_sdk/struct.Address.html#method.require_auth_for_args)

## Testing

Test fixtures demonstrating this vulnerability are available at:

- `contracts/fixtures/finding-codes/s030_require_auth_for_args.rs`

### Example Test

```rust
#[test]
fn test_replay_attack_prevented() {
    let env = Env::default();
    let contract = TestContractClient::new(&env, &env.register_contract(None, TestContract));

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let attacker = Address::generate(&env);

    // Alice authorizes setting Bob as admin
    contract.set_admin(&alice, &bob);

    // Attacker tries to replay with different address
    // This should FAIL if require_auth_for_args is used correctly
    let result = contract.try_set_admin(&alice, &attacker);
    assert!(result.is_err());
}
```

## Configuration

This rule is enabled by default with **High** severity. To customize in `.sanctify.toml`:

```toml
[rules.require_auth_for_args]
enabled = true
severity = "error"  # or "warning", "info"
```

## False Positives

This rule may flag functions where:

- Multiple addresses serve different roles and replay is not a concern
- Additional validation logic prevents replay attacks

In such cases, you can suppress the warning with:

```rust
// sanctifier: ignore[require_auth_for_args]
pub fn special_case(env: Env, addr1: Address, addr2: Address) {
    addr1.require_auth();
    // ... special validation logic
}
```

However, carefully review whether `require_auth_for_args()` would be more secure.

## Summary

Always use `require_auth_for_args()` for functions with multiple `Address` parameters that perform state changes or external calls. This binds authorization to the exact call payload, preventing replay and scope-confusion attacks that could lead to unauthorized operations and privilege escalation.
