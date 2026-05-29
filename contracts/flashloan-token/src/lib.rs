#![no_std]

//! # Flashloan Token — Soroban Reference Implementation
//!
//! A flashloan allows a borrower to take out an uncollateralised loan within a
//! single transaction. The entire loan (plus fee) **must** be repaid before the
//! transaction ends; if the repayment check fails the whole transaction reverts.
//!
//! ## Security invariants
//! 1. **Atomicity** — borrow and repay happen in the same ledger transaction.
//! 2. **Fee enforcement** — the contract verifies `repaid >= borrowed + fee`.
//! 3. **Re-entrancy guard** — a per-instance lock prevents recursive borrows.
//! 4. **Admin auth** — only the designated admin can change the fee rate or pause.
//!
//! ## Storage layout
//! | Key            | Type      | Lifetime   | Description                    |
//! |--------------- |-----------|------------|-------------------------------- |
//! | `ADMIN`        | `Address` | Instance   | Contract administrator          |
//! | `FEE_BPS`      | `u32`     | Instance   | Fee in basis-points (default 9) |
//! | `PAUSED`       | `bool`    | Instance   | Emergency pause flag            |
//! | `FL_LOCK`      | `bool`    | Instance   | Re-entrancy mutex               |
//! | `TOTAL_FEES`   | `i128`    | Persistent | Accumulated fees collected      |
//! | `TOTAL_LOANS`  | `u32`     | Persistent | Total number of flashloans      |

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Bytes, Env, IntoVal, Symbol,
};

// ── Storage keys ────────────────────────────────────────────────────────────────

const ADMIN: Symbol = symbol_short!("ADMIN");
const FEE_BPS: Symbol = symbol_short!("FEE_BPS");
const PAUSED: Symbol = symbol_short!("PAUSED");
const FL_LOCK: Symbol = symbol_short!("FL_LOCK");
const TOTAL_FEES: Symbol = symbol_short!("TOT_FEES");
const TOTAL_LOANS: Symbol = symbol_short!("TOT_LONS");

/// Default fee: 9 basis-points (0.09 %).
const DEFAULT_FEE_BPS: u32 = 9;
/// Maximum allowed fee: 100 bps (1 %).
const MAX_FEE_BPS: u32 = 100;
/// Basis-point denominator.
const BPS_DENOM: i128 = 10_000;

// ── Error codes ─────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum FlashloanError {
    /// Contract is paused by admin.
    Paused = 1,
    /// A flashloan is already in progress (re-entrancy guard).
    Locked = 2,
    /// Requested amount is zero.
    ZeroAmount = 3,
    /// Repayment is less than borrowed + fee.
    InsufficientRepayment = 4,
    /// Fee rate exceeds the allowed maximum.
    FeeTooHigh = 5,
    /// Contract not yet initialised.
    NotInitialised = 6,
}

// ── Contract ────────────────────────────────────────────────────────────────────

#[contract]
pub struct FlashloanToken;

#[contractimpl]
impl FlashloanToken {
    // ── Admin / lifecycle ──────────────────────────────────────────────────────

    /// Initialise the contract. Can only be called once.
    pub fn initialize(env: Env, admin: Address, fee_bps: u32) {
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialised");
        }
        assert!(fee_bps <= MAX_FEE_BPS, "fee exceeds maximum");
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&FEE_BPS, &fee_bps);
        env.storage().instance().set(&PAUSED, &false);
        env.storage().instance().set(&FL_LOCK, &false);
        env.storage().persistent().set(&TOTAL_FEES, &0_i128);
        env.storage().persistent().set(&TOTAL_LOANS, &0_u32);
    }

    /// Change the fee rate. Only callable by admin.
    pub fn set_fee(env: Env, new_fee_bps: u32) {
        Self::require_admin(&env);
        assert!(new_fee_bps <= MAX_FEE_BPS, "fee exceeds maximum");
        env.storage().instance().set(&FEE_BPS, &new_fee_bps);
        env.events()
            .publish((symbol_short!("set_fee"),), new_fee_bps);
    }

    /// Pause / unpause the contract. Only callable by admin.
    pub fn set_paused(env: Env, paused: bool) {
        Self::require_admin(&env);
        env.storage().instance().set(&PAUSED, &paused);
        env.events().publish((symbol_short!("paused"),), paused);
    }

    // ── Core flashloan logic ───────────────────────────────────────────────────

    /// Execute a flashloan.
    ///
    /// Transfers `amount` tokens of `token_address` to `receiver`, then calls
    /// `receiver.execute_operation(token_address, amount, fee, params)` and
    /// finally verifies that the contract's balance has increased by at least
    /// `fee`.
    ///
    /// The receiver contract **must** implement the `IFlashloanReceiver` interface
    /// (i.e. expose an `execute_operation` function that accepts the loan and
    /// repays before returning).
    pub fn flashloan(
        env: Env,
        receiver: Address,
        token_address: Address,
        amount: i128,
        params: Bytes,
    ) -> i128 {
        Self::assert_not_paused(&env);
        Self::acquire_lock(&env);

        assert!(amount > 0, "amount must be positive");

        let fee_bps: u32 = env
            .storage()
            .instance()
            .get(&FEE_BPS)
            .unwrap_or(DEFAULT_FEE_BPS);
        let fee: i128 = amount.checked_mul(fee_bps as i128).expect("fee overflow") / BPS_DENOM;

        let token_client = token::Client::new(&env, &token_address);
        let contract_address = env.current_contract_address();

        // Record balance before lending
        let balance_before = token_client.balance(&contract_address);

        // Transfer funds to receiver
        token_client.transfer(&contract_address, &receiver, &amount);

        // Invoke the receiver's `execute_operation` callback
        env.invoke_contract::<()>(
            &receiver,
            &symbol_short!("exec_op"),
            soroban_sdk::vec![
                &env,
                token_address.into_val(&env),
                amount.into_val(&env),
                fee.into_val(&env),
                params.into_val(&env),
            ],
        );

        // Verify repayment
        let balance_after = token_client.balance(&contract_address);
        let required_repayment = balance_before.checked_add(fee).expect("repayment overflow");

        assert!(
            balance_after >= required_repayment,
            "flashloan not repaid: got {} expected >= {}",
            balance_after,
            required_repayment
        );

        // Accounting
        let collected_fee = balance_after - balance_before;
        let prev_fees: i128 = env.storage().persistent().get(&TOTAL_FEES).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&TOTAL_FEES, &prev_fees.saturating_add(collected_fee));

        let prev_loans: u32 = env.storage().persistent().get(&TOTAL_LOANS).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&TOTAL_LOANS, &prev_loans.saturating_add(1));

        env.events().publish(
            (symbol_short!("flashloan"),),
            (receiver, token_address, amount, collected_fee),
        );

        Self::release_lock(&env);
        collected_fee
    }

    // ── View functions ─────────────────────────────────────────────────────────

    /// Returns the current fee rate in basis-points.
    pub fn fee_bps(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&FEE_BPS)
            .unwrap_or(DEFAULT_FEE_BPS)
    }

    /// Returns whether the contract is paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&PAUSED).unwrap_or(false)
    }

    /// Returns `(total_fees_collected, total_loans_count)`.
    pub fn stats(env: Env) -> (i128, u32) {
        let fees: i128 = env.storage().persistent().get(&TOTAL_FEES).unwrap_or(0);
        let loans: u32 = env.storage().persistent().get(&TOTAL_LOANS).unwrap_or(0);
        (fees, loans)
    }

    // ── Internal helpers ───────────────────────────────────────────────────────

    fn require_admin(env: &Env) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN)
            .expect("not initialised");
        admin.require_auth();
    }

    fn assert_not_paused(env: &Env) {
        let paused: bool = env.storage().instance().get(&PAUSED).unwrap_or(false);
        assert!(!paused, "contract is paused");
    }

    fn acquire_lock(env: &Env) {
        let locked: bool = env.storage().instance().get(&FL_LOCK).unwrap_or(false);
        assert!(!locked, "re-entrancy detected");
        env.storage().instance().set(&FL_LOCK, &true);
    }

    fn release_lock(env: &Env) {
        env.storage().instance().set(&FL_LOCK, &false);
    }
}
