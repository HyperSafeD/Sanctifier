//! NEGATIVE fixture: `transfer_from` never authorizes `spender`, so any caller
//! could move another account's tokens. The suite must flag this as
//! `AuthorizationMismatch`.
#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};

#[contract]
pub struct MissingAuth;

#[contractimpl]
impl MissingAuth {
    pub fn allowance(_e: Env, _from: Address, _spender: Address) -> i128 { 0 }
    pub fn approve(_e: Env, from: Address, _spender: Address, _amount: i128, _exp: u32) { from.require_auth(); }
    pub fn balance(_e: Env, _id: Address) -> i128 { 0 }
    pub fn transfer(_e: Env, from: Address, _to: MuxedAddress, _amount: i128) { from.require_auth(); }
    // No `spender.require_auth()` — the authorization gap.
    pub fn transfer_from(_e: Env, spender: Address, _from: Address, _to: Address, _amount: i128) { let _ = spender; }
    pub fn burn(_e: Env, from: Address, _amount: i128) { from.require_auth(); }
    pub fn burn_from(_e: Env, spender: Address, _from: Address, _amount: i128) { spender.require_auth(); }
    pub fn decimals(_e: Env) -> u32 { 7 }
    pub fn name(e: Env) -> String { String::from_str(&e, "X") }
    pub fn symbol(e: Env) -> String { String::from_str(&e, "X") }
}
