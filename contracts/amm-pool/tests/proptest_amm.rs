#![cfg(test)]

use amm_pool::{calculate_liquidity_mint, calculate_swap_output};
use proptest::prelude::*;

proptest! {
    #[test]
    fn liquidity_mint_never_exceeds_proportional_share(
        reserve_a in 1u128..1_000_000u128,
        reserve_b in 1u128..1_000_000u128,
        amount_a in 1u128..1_000_000u128,
        amount_b in 1u128..1_000_000u128,
        total_supply in 1u128..1_000_000u128,
    ) {
        if let Some(minted) = calculate_liquidity_mint(reserve_a, reserve_b, amount_a, amount_b, total_supply) {
            let ratio_a = amount_a.saturating_mul(total_supply) / reserve_a;
            let ratio_b = amount_b.saturating_mul(total_supply) / reserve_b;
            prop_assert!(minted <= ratio_a.min(ratio_b));
        }
    }

    #[test]
    fn swap_output_never_drains_pool(
        reserve_in in 1u128..1_000_000u128,
        reserve_out in 1u128..1_000_000u128,
        amount_in in 1u128..1_000_000u128,
    ) {
        if let Some(output) = calculate_swap_output(reserve_in, reserve_out, amount_in) {
            prop_assert!(output < reserve_out);
        }
    }

    #[test]
    fn swap_output_is_monotonic_in_amount_in(
        reserve_in in 1u128..100_000u128,
        reserve_out in 1u128..100_000u128,
        amount_small in 1u128..10_000u128,
        amount_large in 1u128..10_000u128,
    ) {
        let lo = amount_small.min(amount_large);
        let hi = amount_small.max(amount_large);
        if let (Some(out_lo), Some(out_hi)) = (calculate_swap_output(reserve_in, reserve_out, lo),
               calculate_swap_output(reserve_in, reserve_out, hi)) {
            prop_assert!(out_hi >= out_lo);
        }
    }

    #[test]
    fn liquidity_mint_returns_none_for_zero_reserves(
        amount_a in 1u128..1_000_000u128,
        amount_b in 1u128..1_000_000u128,
        total_supply in 1u128..1_000_000u128,
    ) {
        prop_assert!(calculate_liquidity_mint(0, 1, amount_a, amount_b, total_supply).is_none());
        prop_assert!(calculate_liquidity_mint(1, 0, amount_a, amount_b, total_supply).is_none());
        prop_assert!(calculate_liquidity_mint(1, 1, amount_a, amount_b, 0).is_none());
    }
}
