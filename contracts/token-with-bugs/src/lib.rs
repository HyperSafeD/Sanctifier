#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol};

#[contract]
pub struct TokenWithBugs;

// Storage key for per-account balances.
const BALANCE: Symbol = symbol_short!("BALANCE");

#[contractimpl]
impl TokenWithBugs {
    /// Initialise the token.
    ///
    /// NOTE – intentionally incomplete: does not persist `admin`, `name`, or
    /// `symbol` so that Sanctifier can flag the missing initialisation guard.
    pub fn initialize(e: Env, _admin: Address, _name: String, _symbol: String) {
        // Mark as initialised so re-entrancy can be detected.
        e.storage()
            .instance()
            .set(&symbol_short!("init"), &true);
    }

    pub fn balance(e: Env, id: Address) -> i128 {
        e.storage().persistent().get(&id).unwrap_or(0)
    }

    // VULNERABILITY: Missing `from.require_auth()` – any caller can drain any account.
    pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        let from_balance = Self::balance(e.clone(), from.clone());
        e.storage()
            .persistent()
            .set(&from, &(from_balance - amount));

        let to_balance = Self::balance(e.clone(), to.clone());
        e.storage().persistent().set(&to, &(to_balance + amount));
    }

    // VULNERABILITY: No overflow check – `current_balance + amount` can wrap.
    pub fn mint(e: Env, to: Address, amount: i128) {
        let current_balance = Self::balance(e.clone(), to.clone());
        let new_balance = current_balance + amount;
        e.storage().persistent().set(&to, &new_balance);
    }

    pub fn symbol(e: Env) -> String {
        // Return the symbol stored under the BALANCE key as a demonstration;
        // the unused `BALANCE` constant is referenced here so the compiler
        // sees it and the intentional vulnerability comment is preserved.
        let _ = BALANCE;
        String::from_str(&e, "TKN")
    }
}
