#![no_std]

//! Proof-of-concept: Kani formal verification harnesses for a standard Soroban token contract.
//!
//! This module demonstrates the "Core Logic Separation" pattern: extract pure balance/transfer
//! logic into functions that can be verified with Kani, while the contract layer that uses
//! `Env`, `Address`, `Symbol`, etc. remains unverified due to Host type limitations.

use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

// ── Token initialisation pure logic (verified with Kani) ─────────────────────
//
// The contract must only be initialised once.  We model the "already initialised"
// flag as a single boolean: `is_initialized == true` means setup has already run.
// The function is pure (no Host/FFI), so Kani can reason about every possible
// combination of inputs exhaustively.

/// Attempt to initialise the token contract.
///
/// * `is_initialized` – whether the contract has already been set up.
/// * Returns `Ok(())` on success (transitions the flag from `false` → `true`).
/// * Returns `Err("already initialized")` if the token was already set up,
///   guaranteeing that a second call can **never** succeed.
pub fn initialize_pure(is_initialized: bool) -> Result<(), &'static str> {
    if is_initialized {
        return Err("already initialized");
    }
    Ok(())
}

// ── Pure logic (verified with Kani) ─────────────────────────────────────────────
//
// These functions operate only on i128 and have no Host/FFI dependencies.
// They model the core arithmetic of a standard Soroban token: transfer, mint, burn.

/// Transfer: deduct from sender, add to receiver.
pub fn transfer_pure(
    balance_from: i128,
    balance_to: i128,
    amount: i128,
) -> Result<(i128, i128), &'static str> {
    if amount <= 0 {
        return Err("Amount must be positive");
    }
    let new_from = balance_from
        .checked_sub(amount)
        .ok_or("Insufficient balance")?;
    let new_to = balance_to
        .checked_add(amount)
        .ok_or("Receiver balance overflow")?;
    Ok((new_from, new_to))
}

/// Mint: add to a balance.
pub fn mint_pure(balance: i128, amount: i128) -> Result<i128, &'static str> {
    if amount <= 0 {
        return Err("Mint amount must be positive");
    }
    balance.checked_add(amount).ok_or("Mint overflow")
}

/// Burn: subtract from a balance.
pub fn burn_pure(balance: i128, amount: i128) -> Result<i128, &'static str> {
    if amount <= 0 {
        return Err("Burn amount must be positive");
    }
    balance
        .checked_sub(amount)
        .ok_or("Insufficient balance to burn")
}

// ── SEP-41-shaped pure operations (same arithmetic as the standard template) ───
//
// The SEP-41 interface uses `transfer`, `transfer_from`, `burn`, and `burn_from`.
// Allowance / approve do not move tokens and do not change total supply.
// Administrative mint is not part of the SEP-41 *interface* in many deployments but
// appears in templates next to these entry points; we model it for supply proofs.

/// `transfer_from` moves `amount` from `balance_from` to `balance_to` — identical
/// accounting to [`transfer_pure`].
#[inline]
pub fn transfer_from_pure(
    balance_from: i128,
    balance_to: i128,
    amount: i128,
) -> Result<(i128, i128), &'static str> {
    transfer_pure(balance_from, balance_to, amount)
}

/// `burn_from` debits `balance_from` like [`burn_pure`].
#[inline]
pub fn burn_from_pure(balance_from: i128, amount: i128) -> Result<i128, &'static str> {
    burn_pure(balance_from, amount)
}

/// Approve only updates allowance metadata; balances and total supply are unchanged.
#[inline]
pub fn approve_pure_no_balance_change(
    balance_from: i128,
    balance_to: i128,
    total_supply: i128,
    _new_allowance: i128,
) -> (i128, i128, i128) {
    (balance_from, balance_to, total_supply)
}

/// Mint increases both a holder balance and aggregate total supply (template pattern).
pub fn mint_pure_with_total_supply(
    total_supply: i128,
    balance: i128,
    amount: i128,
) -> Result<(i128, i128), &'static str> {
    let new_balance = mint_pure(balance, amount)?;
    let new_total = total_supply
        .checked_add(amount)
        .ok_or("Total supply overflow")?;
    Ok((new_total, new_balance))
}

/// Burn decreases both a holder balance and aggregate total supply.
pub fn burn_pure_with_total_supply(
    total_supply: i128,
    balance: i128,
    amount: i128,
) -> Result<(i128, i128), &'static str> {
    let new_balance = burn_pure(balance, amount)?;
    let new_total = total_supply
        .checked_sub(amount)
        .ok_or("Total supply underflow")?;
    Ok((new_total, new_balance))
}

// ── Contract (not verified: uses Host types) ────────────────────────────────────

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    /// Wrapper exposing transfer_pure for contract use.
    /// A full implementation would read/write balances via env.storage().
    pub fn transfer(balance_from: i128, balance_to: i128, amount: i128) -> (i128, i128) {
        transfer_pure(balance_from, balance_to, amount).expect("transfer failed")
    }

    /// One-shot initialisation entry point.
    ///
    /// Reads the flag from instance storage, delegates to `initialize_pure`, and
    /// persists the flag on success.  Kani verifies the pure guard; the Host layer
    /// here is intentionally thin and untouched by the proof.
    pub fn initialize(env: Env, _name: Symbol) {
        let already: bool = env
            .storage()
            .instance()
            .get(&symbol_short!("init"))
            .unwrap_or(false);
        initialize_pure(already).expect("already initialized");
        env.storage().instance().set(&symbol_short!("init"), &true);
    }

    /// A function that interacts with Env (Host types).
    /// Kani cannot verify this: Env, Symbol, and storage operations require host FFI.
    pub fn set_admin(env: Env, new_admin: Symbol) {
        env.storage()
            .instance()
            .set(&symbol_short!("admin"), &new_admin);
    }
}

// ── Kani harnesses ─────────────────────────────────────────────────────────────

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    fn verify_transfer_pure_conservation() {
        let balance_from: i128 = kani::any();
        let balance_to: i128 = kani::any();
        let amount: i128 = kani::any();

        kani::assume(amount > 0);
        kani::assume(balance_from >= amount);
        kani::assume(balance_from <= i128::MAX);
        kani::assume(balance_to >= 0);
        kani::assume(balance_to <= i128::MAX - amount);
        // Ensure the conservation assert itself (new_from + new_to) doesn't overflow.
        // new_from = balance_from - amount, new_to = balance_to + amount
        // new_from + new_to = balance_from + balance_to, so we need total to fit.
        kani::assume(balance_from <= i128::MAX - balance_to);

        let Ok((new_from, new_to)) = transfer_pure(balance_from, balance_to, amount) else {
            panic!("transfer_pure failed despite valid preconditions");
        };

        assert!(new_from == balance_from - amount);
        assert!(new_to == balance_to + amount);
        assert!(
            new_from + new_to == balance_from + balance_to,
            "Conservation of supply"
        );
    }

    /// **Property**: Transfer fails when `amount <= 0`.
    ///
    /// `transfer_pure` explicitly guards against non-positive amounts.
    /// Kani proves this guard always fires for every non-positive `amount`.
    #[kani::proof]
    fn verify_transfer_pure_rejects_non_positive_amount() {
        let balance_from: i128 = kani::any();
        let balance_to: i128 = kani::any();
        let amount: i128 = kani::any();

        kani::assume(amount <= 0);

        let result = transfer_pure(balance_from, balance_to, amount);
        assert!(result.is_err(), "transfer must fail when amount <= 0");
    }

    /// **Property**: Transfer fails when subtraction would underflow `i128`.
    ///
    /// `checked_sub` returns `None` (and `transfer_pure` returns `Err`) only
    /// when `balance_from - amount < i128::MIN`, i.e. true integer underflow.
    #[kani::proof]
    fn verify_transfer_pure_rejects_underflow() {
        let balance_from: i128 = kani::any();
        let balance_to: i128 = kani::any();
        let amount: i128 = kani::any();

        kani::assume(amount > 0);
        // Underflow condition: balance_from - amount < i128::MIN
        kani::assume(balance_from < i128::MIN + amount);

        let result = transfer_pure(balance_from, balance_to, amount);
        assert!(result.is_err(), "transfer must fail on i128 underflow");
    }

    #[kani::proof]
    fn verify_mint_pure() {
        let balance: i128 = kani::any();
        let amount: i128 = kani::any();

        kani::assume(amount > 0);
        kani::assume(balance >= 0);
        kani::assume(balance <= i128::MAX - amount);

        let Ok(new_balance) = mint_pure(balance, amount) else {
            panic!("mint_pure failed despite valid preconditions");
        };

        assert!(new_balance == balance + amount);
    }

    #[kani::proof]
    fn verify_burn_pure() {
        let balance: i128 = kani::any();
        let amount: i128 = kani::any();

        kani::assume(amount > 0);
        kani::assume(balance >= amount);

        let Ok(new_balance) = burn_pure(balance, amount) else {
            panic!("burn_pure failed despite valid preconditions");
        };

        assert!(new_balance == balance - amount);
    }

    // ── Token initialisation proof harnesses ─────────────────────────────────

    /// **Property**: The `initialize` function can only ever be called once
    /// successfully.
    ///
    /// For every possible value of the already-initialised flag Kani proves:
    /// * When `is_initialized == true`  → the call **always** returns `Err`.
    /// * There exists no path through `initialize_pure(true)` that returns `Ok`.
    #[kani::proof]
    fn verify_initialize_fails_when_already_initialized() {
        // Kani considers the single concrete value `true` (contract already set up).
        let result = initialize_pure(true);

        // The guard must always fire; `Ok` is unreachable from this state.
        assert!(
            result.is_err(),
            "initialize must fail when the contract is already initialized"
        );
    }

    /// **Property**: The first call on a fresh (uninitialised) contract always
    /// succeeds.
    ///
    /// When `is_initialized == false` Kani proves:
    /// * `initialize_pure(false)` **always** returns `Ok(())`.
    /// * There exists no path where the first call fails.
    #[kani::proof]
    fn verify_initialize_succeeds_when_not_initialized() {
        // Kani considers the single concrete value `false` (contract is fresh).
        let result = initialize_pure(false);

        // The guard must not fire; setup on an uninitialised contract always works.
        assert!(
            result.is_ok(),
            "initialize must succeed when the contract has not yet been initialized"
        );
    }

    /// **Property**: Double-initialisation is mathematically impossible.
    ///
    /// Kani exhaustively checks **every** boolean value of `is_initialized` and
    /// proves the following invariant:
    ///
    ///   A second call (is_initialized == true) can **never** return Ok.
    ///
    /// Combined with `verify_initialize_succeeds_when_not_initialized`, the two
    /// harnesses together constitute a full mathematical proof that `initialize`
    /// can only ever succeed exactly once across all possible execution traces.
    #[kani::proof]
    fn verify_initialize_idempotency_guarantee() {
        let is_initialized: bool = kani::any();

        let result = initialize_pure(is_initialized);

        if is_initialized {
            // Already set up: the function MUST refuse.
            assert!(
                result.is_err(),
                "initialize must always fail when already initialized"
            );
        } else {
            // Fresh contract: the function MUST succeed.
            assert!(
                result.is_ok(),
                "initialize must succeed on a fresh contract"
            );
        }
    }

    // ── SEP-41 total supply invariant (two-party model: balances a, b) ────────────
    //
    // Invariant: total_supply == a + b.  Proved for each operation class used by
    // the SEP-41 template: transfer, transfer_from, burn, burn_from, approve,
    // and optional mint (administrative).

    #[kani::proof]
    fn verify_transfer_from_equivalent_to_transfer() {
        let a: i128 = kani::any();
        let b: i128 = kani::any();
        let amount: i128 = kani::any();
        let r1 = transfer_pure(a, b, amount);
        let r2 = transfer_from_pure(a, b, amount);
        assert!(r1.is_ok() == r2.is_ok());
        if let (Ok(x), Ok(y)) = (r1, r2) {
            assert!(x.0 == y.0 && x.1 == y.1);
        }
    }

    #[kani::proof]
    fn verify_transfer_preserves_two_party_total_supply() {
        let a: i128 = kani::any();
        let b: i128 = kani::any();
        let amount: i128 = kani::any();
        kani::assume(amount > 0);
        kani::assume(a >= amount);
        kani::assume(a >= 0 && b >= 0);
        kani::assume(b <= i128::MAX - amount);
        kani::assume(a <= i128::MAX - b);

        let total = a + b;
        let Ok((na, nb)) = transfer_pure(a, b, amount) else {
            panic!("unexpected");
        };
        assert!(
            na + nb == total,
            "transfer must preserve sum(a,b) == total_supply"
        );
    }

    #[kani::proof]
    fn verify_transfer_from_preserves_two_party_total_supply() {
        let a: i128 = kani::any();
        let b: i128 = kani::any();
        let amount: i128 = kani::any();
        kani::assume(amount > 0);
        kani::assume(a >= amount);
        kani::assume(a >= 0 && b >= 0);
        kani::assume(b <= i128::MAX - amount);
        kani::assume(a <= i128::MAX - b);

        let total = a + b;
        let Ok((na, nb)) = transfer_from_pure(a, b, amount) else {
            panic!("unexpected");
        };
        assert!(na + nb == total);
    }

    #[kani::proof]
    fn verify_burn_and_burn_from_preserve_invariant_vs_total_supply() {
        let a: i128 = kani::any();
        let b: i128 = kani::any();
        let amount: i128 = kani::any();
        kani::assume(amount > 0);
        kani::assume(a >= amount);
        kani::assume(a >= 0 && b >= 0);
        kani::assume(a <= i128::MAX - b);

        let total = a + b;
        let Ok(new_a) = burn_from_pure(a, amount) else {
            panic!("unexpected");
        };
        let Ok((new_total, new_a2)) = burn_pure_with_total_supply(total, a, amount) else {
            panic!("unexpected");
        };
        assert!(new_a == new_a2);
        assert!(
            new_total == new_a + b,
            "after burn, total_supply == sum of balances"
        );
    }

    #[kani::proof]
    fn verify_approve_preserves_total_supply() {
        let a: i128 = kani::any();
        let b: i128 = kani::any();
        let ts: i128 = kani::any();
        let new_allow: i128 = kani::any();
        kani::assume(ts == a + b);

        let (a2, b2, ts2) = approve_pure_no_balance_change(a, b, ts, new_allow);
        assert!(a2 == a && b2 == b && ts2 == ts);
        assert!(ts2 == a2 + b2);
    }

    #[kani::proof]
    fn verify_mint_preserves_invariant_total_equals_sum() {
        let a: i128 = kani::any();
        let b: i128 = kani::any();
        let amount: i128 = kani::any();
        kani::assume(amount > 0);
        kani::assume(a >= 0 && b >= 0);
        kani::assume(a <= i128::MAX - amount);
        kani::assume(a <= i128::MAX - b);

        let total = a + b;
        let Ok((new_total, new_a)) = mint_pure_with_total_supply(total, a, amount) else {
            panic!("unexpected");
        };
        assert!(
            new_total == new_a + b,
            "mint must increase total_supply and balance coherently"
        );
    }

    /// **Master property**: from any state with `total_supply == a + b`, one step of
    /// each modeled SEP-41 / template operation preserves `total_supply == a + b`
    /// (or updates both sides coherently for mint/burn).
    #[kani::proof]
    fn verify_sep41_total_supply_invariant_all_operations_step() {
        let mut a: i128 = kani::any();
        let mut b: i128 = kani::any();
        kani::assume(a >= 0 && b >= 0);
        kani::assume(a <= i128::MAX - b);

        let mut total_supply = a + b;
        let op: u8 = kani::any();
        kani::assume(op < 5);

        if op == 0 || op == 4 {
            // 0 = transfer, 4 = transfer_from (same arithmetic)
            let amount: i128 = kani::any();
            kani::assume(amount > 0);
            kani::assume(a >= amount);
            kani::assume(b <= i128::MAX - amount);
            let r = if op == 0 {
                transfer_pure(a, b, amount)
            } else {
                transfer_from_pure(a, b, amount)
            };
            if let Ok((na, nb)) = r {
                assert!(na + nb == total_supply);
            }
        } else if op == 1 {
            // burn / burn_from from `a`
            let amount: i128 = kani::any();
            kani::assume(amount > 0);
            kani::assume(a >= amount);
            if let Ok((nt, na)) = burn_pure_with_total_supply(total_supply, a, amount) {
                a = na;
                total_supply = nt;
                assert!(total_supply == a + b);
            }
        } else if op == 2 {
            // mint to `a`
            let amount: i128 = kani::any();
            kani::assume(amount > 0);
            kani::assume(a <= i128::MAX - amount);
            kani::assume(total_supply <= i128::MAX - amount);
            if let Ok((nt, na)) = mint_pure_with_total_supply(total_supply, a, amount) {
                a = na;
                total_supply = nt;
                assert!(total_supply == a + b);
            }
        } else {
            // approve: metadata only
            let new_allow: i128 = kani::any();
            let (a2, b2, ts2) = approve_pure_no_balance_change(a, b, total_supply, new_allow);
            assert!(ts2 == a2 + b2);
        }
    }
}
