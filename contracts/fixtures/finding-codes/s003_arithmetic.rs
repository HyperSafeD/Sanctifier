//! Fixture contracts for finding code **S003 — Arithmetic Overflow / Underflow**.
//!
//! Each `*_bad` function demonstrates a pattern the analyser **must** flag.
//! Each `*_safe` function demonstrates the recommended safe alternative that
//! the analyser **must not** flag.
//!
//! These fixtures are used by integration tests to verify that:
//! 1. The arithmetic overflow rule fires on every unsafe operation below.
//! 2. Safe patterns do not produce false positives.
//! 3. Release-build behaviour is consistent with debug-build behaviour (no
//!    arithmetic overflow fixtures are conditionally compiled away).
//!
//! **Do not add `#[cfg(test)]` guards** — every function must remain visible
//! to the static analyser regardless of build profile.

#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct ArithmeticFixture;

#[contractimpl]
impl ArithmeticFixture {
    // ── Unsafe patterns — analyser MUST flag these ────────────────────────────

    /// S003: Unchecked addition — can overflow on `u32::MAX + 1`.
    pub fn unchecked_add(_env: Env, a: u32, b: u32) -> u32 {
        a + b
    }

    /// S003: Unchecked subtraction — can underflow on `0 - 1`.
    pub fn unchecked_sub(_env: Env, a: u32, b: u32) -> u32 {
        a - b
    }

    /// S003: Unchecked multiplication — can overflow on large values.
    pub fn unchecked_mul(_env: Env, a: u32, b: u32) -> u32 {
        a * b
    }

    /// S003: Compound add-assign — hidden overflow risk in `+=`.
    pub fn unchecked_add_assign(_env: Env, mut balance: i128, delta: i128) -> i128 {
        balance += delta;
        balance
    }

    /// S003: Compound sub-assign — hidden underflow risk in `-=`.
    pub fn unchecked_sub_assign(_env: Env, mut balance: i128, delta: i128) -> i128 {
        balance -= delta;
        balance
    }

    /// S003: Compound mul-assign — hidden overflow in `*=`.
    pub fn unchecked_mul_assign(_env: Env, mut amount: u64, factor: u64) -> u64 {
        amount *= factor;
        amount
    }

    /// S003: mul_div without overflow protection — numerator * denominator
    /// can exceed u128::MAX before the division is applied.
    pub fn unchecked_mul_div(_env: Env, a: i128, b: i128, c: i128) -> i128 {
        a.mul_div(b, c)
    }

    /// S003: fixed_point_mul without overflow protection.
    pub fn unchecked_fixed_point_mul(_env: Env, a: i128, b: i128) -> i128 {
        a.fixed_point_mul(b)
    }

    // ── Safe patterns — analyser MUST NOT flag these ──────────────────────────

    /// Safe: checked_add returns None on overflow.
    pub fn safe_add(_env: Env, a: u32, b: u32) -> Option<u32> {
        a.checked_add(b)
    }

    /// Safe: saturating_add clamps at MAX instead of wrapping.
    pub fn safe_add_saturating(_env: Env, a: u32, b: u32) -> u32 {
        a.saturating_add(b)
    }

    /// Safe: checked_sub returns None on underflow.
    pub fn safe_sub(_env: Env, a: u32, b: u32) -> Option<u32> {
        a.checked_sub(b)
    }

    /// Safe: saturating_sub clamps at 0 instead of wrapping.
    pub fn safe_sub_saturating(_env: Env, a: u32, b: u32) -> u32 {
        a.saturating_sub(b)
    }

    /// Safe: checked_mul returns None on overflow.
    pub fn safe_mul(_env: Env, a: u32, b: u32) -> Option<u32> {
        a.checked_mul(b)
    }

    /// Safe: saturating_mul clamps at MAX instead of wrapping.
    pub fn safe_mul_saturating(_env: Env, a: u32, b: u32) -> u32 {
        a.saturating_mul(b)
    }

    /// Safe: index arithmetic in a subscript is intentional and must not fire.
    pub fn safe_index_arithmetic(_env: Env, buf: &[u8], i: usize) -> u8 {
        buf[i + 1]
    }
}
