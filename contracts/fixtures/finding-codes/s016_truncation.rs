#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Vec};

#[contract]
pub struct TruncationFixture;

#[contractimpl]
impl TruncationFixture {
    pub fn unsafe_cast(_env: Env, large: i128) -> u32 {
        // ❌ RISK: Truncation from i128 to u32 without bounds check.
        large as u32
    }

    pub fn unsafe_index(_env: Env, items: Vec<u32>, index: u32) -> u32 {
        // ❌ RISK: Unchecked array/vec indexing (can panic if out of bounds).
        // Soroban `get` returns an Option, but `get_unchecked` or similar (if used)
        // or just improper handling of indices is flagged.
        items.get(index).unwrap()
    }
}
