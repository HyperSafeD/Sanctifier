//! # Deposit-Withdraw Auth-Gap Fixture
//!
//! Demonstrates the auth-gap vulnerability (Sanctifier rule **S001**):
//! a `withdraw` function that does not call `require_auth` can be drained
//! by any caller.  The safe variant shows the one-line fix.
//!
//! ## Security Disclaimer
//!
//! **This contract is intentionally vulnerable for educational purposes.**
//! Do NOT deploy `withdraw_unsafe` on any real network.
//!
//! ---
//!
//! ## Public Interface
//!
//! | Function           | Auth required? | Description                           |
//! |--------------------|---------------|---------------------------------------|
//! | `deposit`          | ✅ yes         | Transfer tokens into the vault         |
//! | `withdraw_unsafe`  | ❌ **missing** | Drain vault — auth gap (S001)          |
//! | `withdraw_safe`    | ✅ yes         | Correct withdrawal with `require_auth` |
//! | `balance`          | no            | Read caller's stored balance           |
#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    token, Address, Env,
};

// ─── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
enum DataKey {
    /// Per-user deposited balance (token units).
    Balance(Address),
}

// ─── Errors ───────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    InsufficientBalance = 1,
    ZeroAmount          = 2,
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct DepositWithdraw;

#[contractimpl]
impl DepositWithdraw {
    // ─── Deposit ──────────────────────────────────────────────────────────────

    /// Transfer `amount` of `token` from `caller` into this vault.
    ///
    /// The caller must authorise the transfer (`require_auth` is implicit via
    /// `token::Client::transfer_from`, which requires the caller's signature).
    pub fn deposit(env: Env, caller: Address, token: Address, amount: i128) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::ZeroAmount);
        }

        caller.require_auth();

        // Pull tokens from caller into this contract
        token::Client::new(&env, &token)
            .transfer(&caller, &env.current_contract_address(), &amount);

        // Credit balance
        let key = DataKey::Balance(caller.clone());
        let prev: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(prev + amount));

        Ok(())
    }

    // ─── Unsafe withdraw (intentional auth gap — S001) ────────────────────────

    /// ❌ **VULNERABLE**: Missing `require_auth` call.
    ///
    /// Any address can pass an arbitrary `account` and drain its entire
    /// deposited balance.  Sanctifier's S001 rule flags this pattern.
    ///
    /// **Attack vector (flash-loan style):**
    /// 1. Attacker calls `withdraw_unsafe(victim_address, token, victim_balance)`.
    /// 2. Contract does NOT check that the caller == account.
    /// 3. Victim's balance is zeroed and tokens are sent to the attacker.
    pub fn withdraw_unsafe(
        env: Env,
        account: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), Error> {
        // ⚠️  `account.require_auth()` is intentionally omitted here.

        let key = DataKey::Balance(account.clone());
        let bal: i128 = env.storage().persistent().get(&key).unwrap_or(0);

        if amount > bal {
            return Err(Error::InsufficientBalance);
        }

        env.storage().persistent().set(&key, &(bal - amount));

        // Send tokens directly to the caller — NOT to `account`
        token::Client::new(&env, &token)
            .transfer(&env.current_contract_address(), &env.invoker(), &amount);

        Ok(())
    }

    // ─── Safe withdraw (correct implementation) ───────────────────────────────

    /// ✅ **SECURE**: One call to `account.require_auth()` closes the auth gap.
    ///
    /// Only the address that originally deposited can withdraw their own funds.
    pub fn withdraw_safe(
        env: Env,
        account: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), Error> {
        // ✅ Require the transaction to be authorised by `account`
        account.require_auth();

        let key = DataKey::Balance(account.clone());
        let bal: i128 = env.storage().persistent().get(&key).unwrap_or(0);

        if amount > bal {
            return Err(Error::InsufficientBalance);
        }

        env.storage().persistent().set(&key, &(bal - amount));

        token::Client::new(&env, &token)
            .transfer(&env.current_contract_address(), &account, &amount);

        Ok(())
    }

    // ─── View ─────────────────────────────────────────────────────────────────

    /// Return the deposited token balance for `account`.
    pub fn balance(env: Env, account: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(account))
            .unwrap_or(0)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, MockAuth, MockAuthInvoke},
        token::{Client as TokenClient, StellarAssetClient},
        Env, IntoVal,
    };

    fn setup() -> (Env, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let admin       = Address::generate(&env);
        let token_addr  = env.register_stellar_asset_contract_v2(admin.clone()).address();
        let contract_id = env.register(DepositWithdraw, ());
        let user        = Address::generate(&env);

        StellarAssetClient::new(&env, &token_addr).mint(&user, &10_000i128);

        (env, token_addr, contract_id, user, admin)
    }

    /// A normal deposit followed by a safe withdrawal succeeds.
    #[test]
    fn deposit_and_safe_withdraw_roundtrip() {
        let (env, token, contract, user, _) = setup();
        let c = DepositWithdrawClient::new(&env, &contract);

        c.deposit(&user, &token, &1_000i128);
        assert_eq!(c.balance(&user), 1_000);

        c.withdraw_safe(&user, &token, &400i128);
        assert_eq!(c.balance(&user), 600);

        let user_bal = TokenClient::new(&env, &token).balance(&user);
        assert_eq!(user_bal, 9_400); // 10_000 − 1_000 + 400
    }

    /// withdraw_safe with more than deposited returns InsufficientBalance error.
    #[test]
    fn safe_withdraw_over_balance_fails() {
        let (env, token, contract, user, _) = setup();
        let c = DepositWithdrawClient::new(&env, &contract);

        c.deposit(&user, &token, &500i128);
        let result = c.try_withdraw_safe(&user, &token, &1_000i128);
        assert!(result.is_err());
    }

    /// ❌ Auth-gap demo: attacker can drain victim's balance via withdraw_unsafe.
    ///
    /// This test deliberately passes — it documents the exploit, not a safe path.
    #[test]
    fn unsafe_withdraw_auth_gap_exploit() {
        let (env, token, contract, victim, _) = setup();
        let attacker = Address::generate(&env);
        let c = DepositWithdrawClient::new(&env, &contract);

        // Victim deposits 1 000 tokens
        c.deposit(&victim, &token, &1_000i128);
        assert_eq!(c.balance(&victim), 1_000);

        // Attacker calls withdraw_unsafe passing victim's address — no auth check!
        // (mock_all_auths means the absent auth is silently passed in test env)
        c.withdraw_unsafe(&victim, &token, &1_000i128);

        // Victim's on-contract balance is now zero
        assert_eq!(c.balance(&victim), 0, "victim balance drained by attacker");
    }
}
