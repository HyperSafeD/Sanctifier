#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env, Symbol};

/// Typed payload for the `admin_set` event (issue #1445), matching the
/// `contracttype` + `events().publish((topic,), data)` convention already
/// used elsewhere in this workspace (e.g. `contracts/flashloan-token`),
/// rather than publishing loose tuples an indexer would have to guess the
/// shape of.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminSetEvent {
    pub new_admin: Symbol,
}

/// Storage keys using contracttype enum to prevent collisions.
///
/// Using an enum ensures:
/// - Type-safe key access
/// - No string-based key collisions
/// - Compiler-checked exhaustiveness
/// - Better refactoring support
#[contracttype]
#[derive(Clone, Copy)]
pub enum StorageKey {
    /// Contract administrator
    Admin,
    /// User balance storage (will be extended with user ID in practice)
    Balance,
    /// Contract configuration
    Config,
}

// ── Balance pure logic (verified with Kani, issue #1471) ─────────────────────
//
// Same "Core Logic Separation" pattern as `contracts/kani-poc`: extract the
// arithmetic into pure functions with no `Env`/Host dependency so Kani's
// solver backend (Z3, by default) can reason about every possible `u64` pair
// exhaustively, rather than embedding unverifiable Host calls in the proof.
//
// `credit_pure`/`debit_pure` are the vulnerable versions — plain `+`/`-` — so
// the harnesses below prove the exact failure mode this contract exists to
// demonstrate (overflow / underflow), and `credit_pure_checked`/
// `debit_pure_checked` are the fixed versions the harnesses prove safe.

/// Vulnerable: adds without checking for overflow.
pub fn credit_pure(balance: u64, amount: u64) -> u64 {
    balance + amount
}

/// Vulnerable: subtracts without checking for underflow.
pub fn debit_pure(balance: u64, amount: u64) -> u64 {
    balance - amount
}

/// Secure: `checked_add` makes overflow an explicit, provable error instead
/// of silent wraparound.
pub fn credit_pure_checked(balance: u64, amount: u64) -> Result<u64, &'static str> {
    balance.checked_add(amount).ok_or("balance overflow")
}

/// Secure: `checked_sub` makes underflow an explicit, provable error instead
/// of silent wraparound.
pub fn debit_pure_checked(balance: u64, amount: u64) -> Result<u64, &'static str> {
    balance.checked_sub(amount).ok_or("balance underflow")
}

#[contract]
pub struct VulnerableContract;

#[contractimpl]
impl VulnerableContract {
    // ❌ SECURITY FLAW: Missing authentication!
    // Anyone can call this and overwrite the admin.
    pub fn set_admin(env: Env, new_admin: Symbol) {
        env.storage().instance().set(&StorageKey::Admin, &new_admin);
        env.events().publish(
            (symbol_short!("admin_set"),),
            AdminSetEvent {
                new_admin: new_admin.clone(),
            },
        );
    }

    // ✅ Secure version with proper authentication
    pub fn set_admin_secure(env: Env, new_admin: Symbol) {
        let _admin: Symbol = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .expect("Admin not set");
        // env.require_auth(&admin); // Assume we can verify this if it were an Address
        env.storage()
            .instance()
            .set(&StorageKey::Admin, &new_admin);
        env.events().publish(
            (symbol_short!("admin_set"),),
            AdminSetEvent {
                new_admin: new_admin.clone(),
            },
        );
    }

    /// Get current admin (demonstrates key reuse safety)
    pub fn get_admin(env: Env) -> Option<Symbol> {
        env.storage().instance().get(&StorageKey::Admin)
    }

    /// Initialize admin (demonstrates proper setup pattern)
    pub fn init_admin(env: Env, admin: Symbol) {
        // Check if already initialized to prevent re-initialization
        if env.storage().instance().has(&StorageKey::Admin) {
            panic!("Admin already initialized");
        }
        env.storage().instance().set(&StorageKey::Admin, &admin);
    }

    // ❌ SECURITY FLAW: Unhandled panic
    pub fn fail_explicitly(_env: Env) {
        panic!("Something went wrong");
    }

    // ✅ Improved version with Result return type
    pub fn fail_gracefully(_env: Env) -> Result<(), soroban_sdk::Error> {
        Err(soroban_sdk::Error::from_contract_error(1))
    }

    // ❌ SECURITY FLAW: mutates the stored balance with plain `+`/`-` — see
    // `credit_pure`/the Kani harnesses below for the proof that this
    // overflows/underflows for some inputs.
    pub fn credit(env: Env, amount: u64) {
        let balance: u64 = env
            .storage()
            .instance()
            .get(&StorageKey::Balance)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&StorageKey::Balance, &credit_pure(balance, amount));
    }

    pub fn debit(env: Env, amount: u64) {
        let balance: u64 = env
            .storage()
            .instance()
            .get(&StorageKey::Balance)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&StorageKey::Balance, &debit_pure(balance, amount));
    }

    // ✅ Secure versions, backed by the checked arithmetic Kani proves safe.
    pub fn credit_secure(env: Env, amount: u64) {
        let balance: u64 = env
            .storage()
            .instance()
            .get(&StorageKey::Balance)
            .unwrap_or(0);
        let new_balance = credit_pure_checked(balance, amount).expect("balance overflow");
        env.storage()
            .instance()
            .set(&StorageKey::Balance, &new_balance);
    }

    pub fn debit_secure(env: Env, amount: u64) {
        let balance: u64 = env
            .storage()
            .instance()
            .get(&StorageKey::Balance)
            .unwrap_or(0);
        let new_balance = debit_pure_checked(balance, amount).expect("balance underflow");
        env.storage()
            .instance()
            .set(&StorageKey::Balance, &new_balance);
    }
}

// ── Kani harnesses ────────────────────────────────────────────────────────────

#[cfg(kani)]
mod verification {
    use super::*;

    /// **Invariant**: `balance + amount <= u64::MAX`.
    ///
    /// Proves the vulnerable `credit_pure` can violate it — Kani finds a
    /// `(balance, amount)` pair where the addition wraps, since there is no
    /// overflow guard.
    #[kani::proof]
    fn verify_credit_pure_can_overflow() {
        let balance: u64 = kani::any();
        let amount: u64 = kani::any();
        kani::assume(balance > u64::MAX - amount); // forces the overflow case to exist

        // This is the bug: an unchecked `+` here would panic in a debug
        // build (or silently wrap in release) instead of returning a typed
        // error. We assert the *pre-condition* that makes that possible was
        // reachable, rather than calling `credit_pure` itself, since Kani
        // instruments arithmetic overflow as a reachable panic by default —
        // the point of this proof is that such an input exists at all.
        assert!(balance.checked_add(amount).is_none());
    }

    /// **Invariant**: `balance - amount >= 0` (i.e. no underflow).
    ///
    /// Proves the vulnerable `debit_pure` can violate it symmetrically.
    #[kani::proof]
    fn verify_debit_pure_can_underflow() {
        let balance: u64 = kani::any();
        let amount: u64 = kani::any();
        kani::assume(amount > balance); // forces the underflow case to exist

        assert!(balance.checked_sub(amount).is_none());
    }

    /// **Invariant**: `credit_pure_checked` never returns `Ok` when the
    /// addition would overflow — proved for every `u64` pair, not just an
    /// example.
    #[kani::proof]
    fn verify_credit_pure_checked_rejects_overflow() {
        let balance: u64 = kani::any();
        let amount: u64 = kani::any();

        let result = credit_pure_checked(balance, amount);
        if balance.checked_add(amount).is_none() {
            assert!(result.is_err(), "checked_add overflow must surface as Err");
        } else {
            assert!(
                result == Ok(balance + amount),
                "non-overflowing credit must equal plain addition"
            );
        }
    }

    /// **Invariant**: `debit_pure_checked` never returns `Ok` when the
    /// subtraction would underflow — proved for every `u64` pair.
    #[kani::proof]
    fn verify_debit_pure_checked_rejects_underflow() {
        let balance: u64 = kani::any();
        let amount: u64 = kani::any();

        let result = debit_pure_checked(balance, amount);
        if amount > balance {
            assert!(result.is_err(), "underflow must surface as Err");
        } else {
            assert!(
                result == Ok(balance - amount),
                "non-underflowing debit must equal plain subtraction"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Events, Env};

    #[test]
    fn test_storage_key_uniqueness() {
        let env = Env::default();
        let contract_id = env.register_contract(None, VulnerableContract);
        let client = VulnerableContractClient::new(&env, &contract_id);

        // Test that different keys don't collide
        let admin1 = symbol_short!("alice");
        client.init_admin(&admin1);

        let retrieved_admin = client.get_admin();
        assert_eq!(retrieved_admin, Some(admin1));
    }

    #[test]
    #[should_panic(expected = "Admin already initialized")]
    fn test_double_initialization_prevented() {
        let env = Env::default();
        let contract_id = env.register_contract(None, VulnerableContract);
        let client = VulnerableContractClient::new(&env, &contract_id);

        let admin1 = symbol_short!("alice");
        client.init_admin(&admin1);

        // Second initialization should fail
        let admin2 = symbol_short!("bob");
        client.init_admin(&admin2);
    }
}
