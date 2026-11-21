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
//! | `FL_LOCK`      | `bool`    | Temporary  | Re-entrancy mutex (TTL-extended) |
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
/// Re-entrancy guard TTL in ledgers (approx 5 minutes at 5s/ledger).
const LOCK_TTL: u32 = 60;

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

struct FlashloanLock<'a> {
    env: &'a Env,
    active: bool,
}

impl<'a> FlashloanLock<'a> {
    fn new(env: &'a Env) -> Self {
        FlashloanToken::acquire_lock(env);
        Self { env, active: true }
    }

    fn release(&mut self) {
        if self.active {
            FlashloanToken::release_lock(self.env);
            self.active = false;
        }
    }
}

impl Drop for FlashloanLock<'_> {
    fn drop(&mut self) {
        self.release();
    }
}

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
        // FL_LOCK is now stored in temporary storage with TTL, no need to initialize
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
        let mut lock = FlashloanLock::new(&env);

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
        let callback_result = env.try_invoke_contract::<(), soroban_sdk::Error>(
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
        if callback_result.is_err() {
            lock.release();
            panic!("flashloan callback failed");
        }

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

        lock.release();
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

    /// Take the re-entrancy mutex.
    ///
    /// The guard lives in temporary storage so it is scoped to the transaction
    /// and can never be resurrected by a stale ledger entry, and its TTL is
    /// bumped on every acquisition so it cannot lapse between the guard being
    /// set and the loan being repaid.
    fn acquire_lock(env: &Env) {
        let storage = env.storage().temporary();
        let locked: bool = storage.get(&FL_LOCK).unwrap_or(false);
        assert!(!locked, "re-entrancy detected");
        storage.set(&FL_LOCK, &true);
        storage.extend_ttl(&FL_LOCK, LOCK_TTL, LOCK_TTL);
    }

    fn release_lock(env: &Env) {
        env.storage().temporary().set(&FL_LOCK, &false);
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use soroban_sdk::{
        symbol_short,
        testutils::{storage::Temporary as _, Address as _, Ledger as _},
        Address, Bytes, Env,
    };

    #[soroban_sdk::contract]
    pub struct MockToken;

    #[soroban_sdk::contractimpl]
    impl MockToken {
        pub fn balance(e: Env, addr: Address) -> i128 {
            e.storage().instance().get(&addr).unwrap_or(0i128)
        }
        pub fn set_balance(e: Env, addr: Address, amount: i128) {
            e.storage().instance().set(&addr, &amount);
        }
        pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
            let from_bal: i128 = e.storage().instance().get(&from).unwrap_or(0);
            let to_bal: i128 = e.storage().instance().get(&to).unwrap_or(0);
            e.storage().instance().set(&from, &(from_bal - amount));
            e.storage().instance().set(&to, &(to_bal + amount));
        }
        pub fn total_supply(_e: Env) -> i128 {
            1_000_000
        }
    }

    #[soroban_sdk::contract]
    pub struct GoodReceiver;

    #[soroban_sdk::contractimpl]
    impl GoodReceiver {
        pub fn set_lender(e: Env, lender: Address) {
            e.storage()
                .instance()
                .set(&symbol_short!("LENDER"), &lender);
        }

        pub fn exec_op(e: Env, token: Address, amount: i128, fee: i128, _params: Bytes) {
            let lender: Address = e
                .storage()
                .instance()
                .get(&symbol_short!("LENDER"))
                .unwrap();
            let receiver = e.current_contract_address();
            let token_client = soroban_sdk::token::Client::new(&e, &token);
            token_client.transfer(&receiver, &lender, &(amount + fee));
        }
    }

    #[soroban_sdk::contract]
    pub struct BadReceiver;

    #[soroban_sdk::contractimpl]
    impl BadReceiver {
        pub fn fail_op(_e: Env, _token: Address, _amount: i128, _fee: i128, _params: Bytes) {
            panic!("callback failed intentionally");
        }
    }

    /// Receiver that calls back into the lender from inside `exec_op`.
    ///
    /// It records whether the nested borrow was allowed and then repays the
    /// outer loan honestly so the surrounding flashloan can complete. It lives
    /// in its own module because the generated contract-spec identifiers are
    /// not namespaced per contract.
    mod reentrant {
        use super::*;

        #[soroban_sdk::contract]
        pub struct ReentrantReceiver;

        #[soroban_sdk::contractimpl]
        impl ReentrantReceiver {
            pub fn set_lender(e: Env, lender: Address) {
                e.storage()
                    .instance()
                    .set(&symbol_short!("LENDER"), &lender);
            }

            /// Whether the re-entrant borrow was allowed to proceed.
            pub fn nested_borrow_succeeded(e: Env) -> bool {
                e.storage()
                    .instance()
                    .get(&symbol_short!("NESTED"))
                    .unwrap_or(false)
            }

            pub fn exec_op(e: Env, token: Address, amount: i128, fee: i128, _params: Bytes) {
                let lender: Address = e
                    .storage()
                    .instance()
                    .get(&symbol_short!("LENDER"))
                    .unwrap();

                let nested = FlashloanTokenClient::new(&e, &lender).try_flashloan(
                    &lender,
                    &token,
                    &(amount / 2),
                    &Bytes::new(&e),
                );
                let nested_ok = nested.is_ok();
                e.storage()
                    .instance()
                    .set(&symbol_short!("NESTED"), &nested_ok);

                let receiver = e.current_contract_address();
                let token_client = soroban_sdk::token::Client::new(&e, &token);
                token_client.transfer(&receiver, &lender, &(amount + fee));
            }
        }
    }

    use reentrant::{ReentrantReceiver, ReentrantReceiverClient};

    /// Throwaway temporary key used to measure the *default* temporary-entry TTL
    /// so the guard's bump can be compared against it.
    const PROBE_KEY: Symbol = symbol_short!("PROBE");

    #[test]
    fn test_flashloan_success() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let receiver = env.register_contract(None, GoodReceiver);
        let token_addr = env.register_contract(None, MockToken);
        let flashloan_addr = env.register_contract(None, FlashloanToken);

        let token_client = MockTokenClient::new(&env, &token_addr);
        token_client.set_balance(&flashloan_addr, &10_000);

        let client = FlashloanTokenClient::new(&env, &flashloan_addr);
        client.initialize(&admin, &9);
        GoodReceiverClient::new(&env, &receiver).set_lender(&flashloan_addr);

        let fee = client.flashloan(&receiver, &token_addr, &1000, &Bytes::new(&env));
        assert_eq!(fee, 0); // 1000 * 9 / 10000 = 0 (integer division)
    }

    #[test]
    fn test_failed_callback_does_not_lock_contract() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let receiver = env.register_contract(None, BadReceiver);
        let token_addr = env.register_contract(None, MockToken);
        let flashloan_addr = env.register_contract(None, FlashloanToken);

        let token_client = MockTokenClient::new(&env, &token_addr);
        token_client.set_balance(&flashloan_addr, &10_000);

        let client = FlashloanTokenClient::new(&env, &flashloan_addr);
        client.initialize(&admin, &9);

        // First flashloan with bad receiver should fail
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.flashloan(&receiver, &token_addr, &1000, &Bytes::new(&env));
        }));
        assert!(result.is_err(), "flashloan should have failed");

        // Second flashloan with good receiver should succeed (lock not permanently stuck)
        let good_receiver = env.register_contract(None, GoodReceiver);
        GoodReceiverClient::new(&env, &good_receiver).set_lender(&flashloan_addr);
        let fee = client.flashloan(&good_receiver, &token_addr, &1000, &Bytes::new(&env));
        assert_eq!(fee, 0);
    }

    #[test]
    fn test_reentrancy_blocked() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_addr = env.register_contract(None, MockToken);
        let flashloan_addr = env.register_contract(None, FlashloanToken);

        let token_client = MockTokenClient::new(&env, &token_addr);
        token_client.set_balance(&flashloan_addr, &10_000);

        let client = FlashloanTokenClient::new(&env, &flashloan_addr);
        client.initialize(&admin, &9);

        // This test verifies the lock is acquired and released correctly around
        // a well-behaved receiver; `test_reentrant_receiver_is_rejected` covers
        // the actual re-entrancy path.
        let receiver = env.register_contract(None, GoodReceiver);
        GoodReceiverClient::new(&env, &receiver).set_lender(&flashloan_addr);
        let fee = client.flashloan(&receiver, &token_addr, &1000, &Bytes::new(&env));
        assert_eq!(fee, 0);
    }

    /// A receiver that calls `flashloan` again from inside the callback must be
    /// rejected: the guard has to still be held while the loan is outstanding.
    #[test]
    fn test_reentrant_receiver_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let receiver = env.register_contract(None, ReentrantReceiver);
        let token_addr = env.register_contract(None, MockToken);
        let flashloan_addr = env.register_contract(None, FlashloanToken);

        let token_client = MockTokenClient::new(&env, &token_addr);
        token_client.set_balance(&flashloan_addr, &10_000);

        let client = FlashloanTokenClient::new(&env, &flashloan_addr);
        client.initialize(&admin, &9);
        ReentrantReceiverClient::new(&env, &receiver).set_lender(&flashloan_addr);

        let fee = client.flashloan(&receiver, &token_addr, &1000, &Bytes::new(&env));
        assert_eq!(fee, 0);

        assert!(
            !ReentrantReceiverClient::new(&env, &receiver).nested_borrow_succeeded(),
            "re-entrant flashloan was allowed while the outer loan was outstanding"
        );

        // The rejected re-entrant borrow must not have moved any funds.
        assert_eq!(token_client.balance(&flashloan_addr), 10_000);
    }

    /// S025 regression: the guard must have its TTL bumped on acquisition, and
    /// must live in temporary storage so it cannot be lost to a ledger-TTL
    /// expiry between the guard being set and the loan being repaid.
    #[test]
    fn test_reentrancy_guard_ttl_is_bumped() {
        let env = Env::default();
        let flashloan_addr = env.register_contract(None, FlashloanToken);

        // Default TTL for a freshly written temporary entry, with no bump.
        let baseline: u32 = env.as_contract(&flashloan_addr, || {
            env.storage().temporary().set(&PROBE_KEY, &true);
            env.storage().temporary().get_ttl(&PROBE_KEY)
        });

        env.as_contract(&flashloan_addr, || {
            FlashloanToken::acquire_lock(&env);
        });

        let ttl: u32 = env.as_contract(&flashloan_addr, || {
            env.storage().temporary().get_ttl(&FL_LOCK)
        });
        assert_eq!(ttl, LOCK_TTL);
        assert!(
            ttl > baseline,
            "guard TTL was not extended past the default"
        );

        // The guard must not be reachable through ledger-persistent storage.
        assert!(!env.as_contract(&flashloan_addr, || env.storage().instance().has(&FL_LOCK)));
        assert!(!env.as_contract(&flashloan_addr, || env.storage().persistent().has(&FL_LOCK)));
    }

    /// S025 regression: with the guard stored in instance storage and no TTL
    /// bump, the entry lapses after the default temporary TTL and re-entrancy
    /// becomes possible. Simulate the ledger crossing that boundary and assert
    /// the guard is still held.
    #[test]
    fn test_guard_persists_across_simulated_ttl_boundary() {
        let env = Env::default();
        let flashloan_addr = env.register_contract(None, FlashloanToken);

        // TTL the guard would have had without the `extend_ttl` bump.
        let unbumped_ttl: u32 = env.as_contract(&flashloan_addr, || {
            env.storage().temporary().set(&PROBE_KEY, &true);
            env.storage().temporary().get_ttl(&PROBE_KEY)
        });
        assert!(
            unbumped_ttl < LOCK_TTL,
            "test premise: bump must widen the TTL"
        );

        env.as_contract(&flashloan_addr, || {
            FlashloanToken::acquire_lock(&env);
        });

        // Well past the unbumped TTL: the guard must still be set.
        env.ledger().set_sequence_number(unbumped_ttl + 5);
        let still_locked: bool = env.as_contract(&flashloan_addr, || {
            env.storage().temporary().get(&FL_LOCK).unwrap_or(false)
        });
        assert!(
            still_locked,
            "re-entrancy guard lapsed at ledger {} before the extended TTL of {LOCK_TTL}",
            unbumped_ttl + 5
        );

        // ...and right up against the extended TTL boundary.
        env.ledger().set_sequence_number(LOCK_TTL);
        let ttl_at_boundary: u32 = env.as_contract(&flashloan_addr, || {
            env.storage().temporary().get_ttl(&FL_LOCK)
        });
        assert_eq!(ttl_at_boundary, 0);
        let locked_at_boundary: bool = env.as_contract(&flashloan_addr, || {
            env.storage().temporary().get(&FL_LOCK).unwrap_or(false)
        });
        assert!(locked_at_boundary, "guard lapsed before its extended TTL");

        // The boundary is real, not vacuously satisfied: one ledger later the
        // entry is genuinely gone. A Soroban transaction cannot span ledgers, so
        // the guard is always held for the whole borrow-and-repay window.
        env.ledger().set_sequence_number(LOCK_TTL + 1);
        let locked_after_boundary: bool = env.as_contract(&flashloan_addr, || {
            env.storage().temporary().get(&FL_LOCK).unwrap_or(false)
        });
        assert!(
            !locked_after_boundary,
            "guard outlived its extended TTL; the boundary assertion is vacuous"
        );
    }
}
