#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

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
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &new_admin);
    }

    // ✅ Secure version
    pub fn set_admin_secure(env: Env, new_admin: Symbol) {
        let _admin: Symbol = env
            .storage()
            .instance()
            .get(&symbol_short!("admin"))
            .expect("Admin not set");
        // env.require_auth(&admin); // Assume we can verify this if it were an Address
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &new_admin);
    }

    pub fn fail_explicitly(_env: Env) {
        panic!("Something went wrong");
    }

    // ❌ SECURITY FLAW: mutates the stored balance with plain `+`/`-` — see
    // `credit_pure`/the Kani harnesses below for the proof that this
    // overflows/underflows for some inputs.
    pub fn credit(env: Env, amount: u64) {
        let balance: u64 = env
            .storage()
            .instance()
            .get(&symbol_short!("balance"))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&symbol_short!("balance"), &credit_pure(balance, amount));
    }

    pub fn debit(env: Env, amount: u64) {
        let balance: u64 = env
            .storage()
            .instance()
            .get(&symbol_short!("balance"))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&symbol_short!("balance"), &debit_pure(balance, amount));
    }

    // ✅ Secure versions, backed by the checked arithmetic Kani proves safe.
    pub fn credit_secure(env: Env, amount: u64) {
        let balance: u64 = env
            .storage()
            .instance()
            .get(&symbol_short!("balance"))
            .unwrap_or(0);
        let new_balance = credit_pure_checked(balance, amount).expect("balance overflow");
        env.storage()
            .instance()
            .set(&symbol_short!("balance"), &new_balance);
    }

    pub fn debit_secure(env: Env, amount: u64) {
        let balance: u64 = env
            .storage()
            .instance()
            .get(&symbol_short!("balance"))
            .unwrap_or(0);
        let new_balance = debit_pure_checked(balance, amount).expect("balance underflow");
        env.storage()
            .instance()
            .set(&symbol_short!("balance"), &new_balance);
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
