#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

/// The smallest valid Soroban contract — one public function, no storage, no auth.
#[contract]
pub struct MinimalContract;

#[contractimpl]
impl MinimalContract {
    pub fn ping(_env: Env) -> u32 {
        42
    }
}
