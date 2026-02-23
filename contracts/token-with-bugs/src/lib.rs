#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, String, Address, Symbol, Val};

#[contract]
pub struct TokenWithBugs;

const BALANCE: Symbol = symbol_short!("BALANCE");

#[contractimpl]
impl TokenWithBugs {
    pub fn initialize(e: Env, admin: Address, name: String, symbol: String) {
        // Not implemented for this test
    }

    pub fn balance(e: Env, id: Address) -> i128 {
        e.storage().persistent().get(&id).unwrap_or(0)
    }

    // This transfer function is missing an authorization check but performs a storage operation
    pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        // Vulnerability: Missing require_auth call for 'from'
        let from_balance = Self::balance(e.clone(), from.clone());
        e.storage().persistent().set(&from, &(from_balance - amount)); // Mutable operation
        
        let to_balance = Self::balance(e.clone(), to.clone());
        e.storage().persistent().set(&to, &(to_balance + amount));
    }

    // This mint function can cause an overflow
    pub fn mint(e: Env, to: Address, amount: i128) {
        // VULNERABILITY: No overflow check
        let current_balance = Self::balance(e.clone(), to.clone());
        let new_balance = current_balance + amount; // This can overflow
        e.storage().persistent().set(&to, &new_balance);
    }

    pub fn symbol(e: Env) -> String {
        String::from_str(&e, "TKN")
    }
}
