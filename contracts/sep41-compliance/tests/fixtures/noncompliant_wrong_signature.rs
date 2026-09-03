//! NEGATIVE fixture: `transfer` takes `amount: i64` instead of `i128`, and
//! `balance` returns `u64` instead of `i128`. The suite must flag these as
//! `SignatureMismatch`.
#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, MuxedAddress, String};

#[contract]
pub struct WrongSig;

#[contractimpl]
impl WrongSig {
    pub fn allowance(_e: Env, _from: Address, _spender: Address) -> i128 { 0 }
    pub fn approve(_e: Env, from: Address, _spender: Address, _amount: i128, _exp: u32) { from.require_auth(); }
    pub fn balance(_e: Env, _id: Address) -> u64 { 0 } // wrong return type
    pub fn transfer(_e: Env, from: Address, _to: MuxedAddress, _amount: i64) { from.require_auth(); } // wrong amount type
    pub fn transfer_from(_e: Env, spender: Address, _from: Address, _to: Address, _amount: i128) { spender.require_auth(); }
    pub fn burn(_e: Env, from: Address, _amount: i128) { from.require_auth(); }
    pub fn burn_from(_e: Env, spender: Address, _from: Address, _amount: i128) { spender.require_auth(); }
    pub fn decimals(_e: Env) -> u32 { 7 }
    pub fn name(e: Env) -> String { String::from_str(&e, "X") }
    pub fn symbol(e: Env) -> String { String::from_str(&e, "X") }
}
