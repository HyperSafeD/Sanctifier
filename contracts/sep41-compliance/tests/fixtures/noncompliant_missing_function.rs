//! NEGATIVE fixture: a token that is missing the `burn_from` SEP-41 function.
//! The compliance suite must flag this as `MissingFunction`.
#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};

#[contract]
pub struct MissingFn;

#[contractimpl]
impl MissingFn {
    pub fn allowance(_e: Env, _from: Address, _spender: Address) -> i128 { 0 }
    pub fn approve(_e: Env, from: Address, _spender: Address, _amount: i128, _exp: u32) { from.require_auth(); }
    pub fn balance(_e: Env, _id: Address) -> i128 { 0 }
    pub fn transfer(_e: Env, from: Address, _to: MuxedAddress, _amount: i128) { from.require_auth(); }
    pub fn transfer_from(_e: Env, spender: Address, _from: Address, _to: Address, _amount: i128) { spender.require_auth(); }
    pub fn burn(_e: Env, from: Address, _amount: i128) { from.require_auth(); }
    // `burn_from` intentionally omitted.
    pub fn decimals(_e: Env) -> u32 { 7 }
    pub fn name(e: Env) -> String { String::from_str(&e, "X") }
    pub fn symbol(e: Env) -> String { String::from_str(&e, "X") }
}
