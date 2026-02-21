#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol, symbol_short};

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    /// A simple transfer function that updates balances.
    /// This is a simplified version for Kani verification PoC.
    /// In a real contract, we would use storage.
    pub fn transfer_pure(balance_from: i128, balance_to: i128, amount: i128) -> (i128, i128) {
        if amount < 0 {
            panic!("Negative amount");
        }
        if balance_from < amount {
            panic!("Insufficient balance");
        }
        let new_from = balance_from - amount;
        let new_to = balance_to + amount;
        (new_from, new_to)
    }

    /// A function that interacts with Env (Host types).
    /// This is where Kani verification becomes challenging.
    pub fn set_admin(env: Env, new_admin: Symbol) {
        env.storage().instance().set(&symbol_short!("admin"), &new_admin);
    }
}

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    pub fn verify_transfer_pure() {
        let balance_from: i128 = kani::any();
        let balance_to: i128 = kani::any();
        let amount: i128 = kani::any();

        // Preconditions
        kani::assume(amount >= 0);
        kani::assume(balance_from >= amount);
        kani::assume(balance_to >= 0);
        // Avoid overflow for the receiver
        kani::assume(balance_to <= i128::MAX - amount);

        let (new_from, new_to) = TokenContract::transfer_pure(balance_from, balance_to, amount);

        // Postconditions
        assert!(new_from == balance_from - amount);
        assert!(new_to == balance_to + amount);
        assert!(new_from + new_to == balance_from + balance_to);
    }
}
