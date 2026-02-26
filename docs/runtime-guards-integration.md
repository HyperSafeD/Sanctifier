# Integrating Sanctifier Runtime Guards in Soroban

This guide shows how to add runtime validation wrappers to an existing Soroban contract using Sanctifier's runtime guard trait.

## What you will use

- `sanctifier_core::SanctifiedGuard`
- `sanctifier_core::Error` (runtime invariant error type)
- Your existing `soroban_sdk::Env` storage reads

## 1. Add the dependency

In your contract crate `Cargo.toml`:

```toml
[dependencies]
sanctifier-core = { path = "../../tooling/sanctifier-core" }
soroban-sdk = "20.5.0"
```

## 2. Define guard context and invariant checker

Create a runtime guard type that inspects current on-chain state and enforces invariants.

```rust
use sanctifier_core::{Error as SanctifierError, SanctifiedGuard};
use soroban_sdk::{Address, Env, Symbol};

pub struct VaultRuntimeGuard {
    pub owner: Address,
    pub total_shares: i128,
    pub assets_under_management: i128,
}

impl SanctifiedGuard for VaultRuntimeGuard {
    fn check_invariant(&self, _env: &Env) -> Result<(), SanctifierError> {
        if self.total_shares < 0 || self.assets_under_management < 0 {
            return Err(SanctifierError::InvariantViolation(
                "negative balances are not allowed".to_string(),
            ));
        }

        if self.total_shares == 0 && self.assets_under_management != 0 {
            return Err(SanctifierError::InvariantViolation(
                "assets must be zero when total shares is zero".to_string(),
            ));
        }

        Ok(())
    }
}
```

## 3. Add a wrapper helper around state-mutating entrypoints

Use a small helper that runs guard checks before and after mutation logic.

```rust
fn with_runtime_guard<F>(env: &Env, f: F) -> Result<(), SanctifierError>
where
    F: FnOnce(&Env) -> Result<(), SanctifierError>,
{
    let guard_before = load_guard(env);
    guard_before.check_invariant(env)?;

    f(env)?;

    let guard_after = load_guard(env);
    guard_after.check_invariant(env)?;
    Ok(())
}

fn load_guard(env: &Env) -> VaultRuntimeGuard {
    let owner: Address = env.storage().instance().get(&Symbol::new(env, "OWNER")).unwrap();
    let total_shares: i128 = env
        .storage()
        .instance()
        .get(&Symbol::new(env, "TOTAL_SHARES"))
        .unwrap_or(0);
    let assets_under_management: i128 = env
        .storage()
        .instance()
        .get(&Symbol::new(env, "AUM"))
        .unwrap_or(0);

    VaultRuntimeGuard {
        owner,
        total_shares,
        assets_under_management,
    }
}
```

## 4. Apply wrapper in your contract functions

```rust
#[contractimpl]
impl VaultContract {
    pub fn deposit(env: Env, caller: Address, amount: i128) -> Result<(), SanctifierError> {
        caller.require_auth();

        with_runtime_guard(&env, |env_ref| {
            // Existing mutation logic
            // update balances, shares, and state safely
            let _ = env_ref;
            Ok(())
        })
    }
}
```

## 5. Recommended invariant checklist

- No negative balances/shares
- Total supply consistency (sum of balances == recorded total where applicable)
- Ownership/admin addresses are initialized and non-empty
- Any capped values (fees, rates, utilization) remain inside allowed bounds

## 6. Pair runtime guards with static/formal checks

Use all three together:

1. Static scan: `sanctifier analyze ./contracts/my-contract`
2. Runtime guard wrappers: before/after state mutation checks
3. Formal proofs (Kani): prove pure logic invariants on critical paths

## Notes

- Runtime guards catch invariant drift at execution time.
- Keep guards focused and deterministic.
- Place heavy business logic in pure helpers and keep contract entrypoints thin.
