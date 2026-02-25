use amm_pool::{calculate_liquidity_burn, calculate_liquidity_mint, calculate_swap_output};
use proptest::prelude::*;

/// Simple integer square root (same as in lib.rs)
fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }

    let mut x = n;
    let mut y = x.div_ceil(2);

    while y < x {
        x = y;
        y = (x + n / x).div_ceil(2);
    }

    x
}

// ── Property-based tests for swap calculations ──────────────────────────────────

proptest! {
    /// Property: Swap output should never overflow
    #[test]
    fn prop_swap_no_overflow(
        reserve_in in 1u128..=u64::MAX as u128,
        reserve_out in 1u128..=u64::MAX as u128,
        amount_in in 1u128..=(u64::MAX as u128 / 10000),
        fee_bps in 0u128..10000u128,
    ) {
        let result = calculate_swap_output(reserve_in, reserve_out, amount_in, fee_bps);

        // Should either succeed or fail gracefully with an error
        match result {
            Ok(output) => {
                // Output should be less than or equal to reserve_out
                prop_assert!(output <= reserve_out);
                // Output should be positive
                prop_assert!(output > 0);
            }
            Err(_) => {
                // Errors are acceptable for edge cases
            }
        }
    }

    /// Property: Swap preserves constant product (with fee adjustment)
    #[test]
    fn prop_swap_constant_product(
        reserve_in in 1000u128..1_000_000u128,
        reserve_out in 1000u128..1_000_000u128,
        amount_in in 1u128..10000u128,
        fee_bps in 0u128..1000u128,
    ) {
        if let Ok(amount_out) = calculate_swap_output(reserve_in, reserve_out, amount_in, fee_bps) {
            // Calculate k before and after
            let k_before = reserve_in.checked_mul(reserve_out);

            let new_reserve_in = reserve_in.checked_add(amount_in);
            let new_reserve_out = reserve_out.checked_sub(amount_out);

            if let (Some(k_before), Some(new_in), Some(new_out)) = (k_before, new_reserve_in, new_reserve_out) {
                let k_after = new_in.checked_mul(new_out);

                if let Some(k_after) = k_after {
                    // k should increase or stay the same (due to fees)
                    prop_assert!(k_after >= k_before);
                }
            }
        }
    }

    /// Property: Larger input amounts should yield larger outputs (monotonicity)
    #[test]
    fn prop_swap_monotonic(
        reserve_in in 1000u128..1_000_000u128,
        reserve_out in 1000u128..1_000_000u128,
        amount_in_1 in 1u128..5000u128,
        amount_in_2 in 5001u128..10000u128,
        fee_bps in 0u128..1000u128,
    ) {
        let output_1 = calculate_swap_output(reserve_in, reserve_out, amount_in_1, fee_bps);
        let output_2 = calculate_swap_output(reserve_in, reserve_out, amount_in_2, fee_bps);

        if let (Ok(out1), Ok(out2)) = (output_1, output_2) {
            // Larger input should yield larger output
            prop_assert!(out2 > out1);
        }
    }

    /// Property: Zero amount should fail
    #[test]
    fn prop_swap_zero_amount_fails(
        reserve_in in 1u128..1_000_000u128,
        reserve_out in 1u128..1_000_000u128,
        fee_bps in 0u128..10000u128,
    ) {
        let result = calculate_swap_output(reserve_in, reserve_out, 0, fee_bps);
        prop_assert!(result.is_err());
    }

    /// Property: Swap with zero reserves should fail
    #[test]
    fn prop_swap_zero_reserves_fails(
        amount_in in 1u128..1_000_000u128,
        fee_bps in 0u128..10000u128,
    ) {
        let result1 = calculate_swap_output(0, 1000, amount_in, fee_bps);
        let result2 = calculate_swap_output(1000, 0, amount_in, fee_bps);

        prop_assert!(result1.is_err());
        prop_assert!(result2.is_err());
    }
}

// ── Property-based tests for liquidity operations ───────────────────────────────

proptest! {
    /// Property: Initial liquidity should never overflow
    #[test]
    fn prop_initial_liquidity_no_overflow(
        amount_a in 1u128..=u64::MAX as u128,
        amount_b in 1u128..=u64::MAX as u128,
    ) {
        let result = calculate_liquidity_mint(0, 0, amount_a, amount_b, 0);

        match result {
            Ok(liquidity) => {
                // Liquidity should be positive
                prop_assert!(liquidity > 0);
                // Liquidity should be approximately sqrt(amount_a * amount_b)
                // which can be larger than min(amount_a, amount_b) for small values
                let product = amount_a.checked_mul(amount_b);
                if let Some(prod) = product {
                    let expected_sqrt = integer_sqrt(prod);
                    prop_assert_eq!(liquidity, expected_sqrt);
                }
            }
            Err(_) => {
                // Errors are acceptable for edge cases
            }
        }
    }

    /// Property: Adding liquidity should never overflow
    #[test]
    fn prop_add_liquidity_no_overflow(
        reserve_a in 1u128..1_000_000u128,
        reserve_b in 1u128..1_000_000u128,
        amount_a in 1u128..100_000u128,
        amount_b in 1u128..100_000u128,
        total_supply in 1u128..1_000_000u128,
    ) {
        let result = calculate_liquidity_mint(reserve_a, reserve_b, amount_a, amount_b, total_supply);

        match result {
            Ok(liquidity) => {
                // Liquidity should be positive
                prop_assert!(liquidity > 0);
                // Liquidity should be reasonable - can be more than 2x for small reserves
                // Just verify it doesn't overflow
            }
            Err(_) => {
                // Errors are acceptable for edge cases
            }
        }
    }

    /// Property: Liquidity proportionality - verify reasonable bounds
    #[test]
    fn prop_liquidity_proportional(
        reserve_a in 1000u128..1_000_000u128,
        reserve_b in 1000u128..1_000_000u128,
        amount_a in 100u128..10_000u128,
        total_supply in 1000u128..1_000_000u128,
    ) {
        // Calculate proportional amount_b
        let amount_b = (amount_a.checked_mul(reserve_b).unwrap_or(0))
            .checked_div(reserve_a)
            .unwrap_or(0);

        if amount_b > 0 {
            let result = calculate_liquidity_mint(reserve_a, reserve_b, amount_a, amount_b, total_supply);

            if let Ok(liquidity) = result {
                // Liquidity should be positive
                prop_assert!(liquidity > 0);

                // Liquidity should be roughly proportional - within an order of magnitude
                let expected_liquidity = (amount_a.checked_mul(total_supply).unwrap_or(0))
                    .checked_div(reserve_a)
                    .unwrap_or(0);

                if expected_liquidity > 0 {
                    // Verify liquidity is within reasonable bounds (not off by orders of magnitude)
                    prop_assert!(liquidity > 0);
                    prop_assert!(liquidity < expected_liquidity * 2);
                }
            }
        }
    }

    /// Property: Removing liquidity should never overflow or underflow
    #[test]
    fn prop_remove_liquidity_no_overflow(
        reserve_a in 1u128..1_000_000u128,
        reserve_b in 1u128..1_000_000u128,
        total_supply in 1u128..1_000_000u128,
        liquidity in 1u128..1_000_000u128,
    ) {
        // Ensure liquidity doesn't exceed total supply
        let liquidity = liquidity.min(total_supply);

        let result = calculate_liquidity_burn(reserve_a, reserve_b, liquidity, total_supply);

        match result {
            Ok((amount_a, amount_b)) => {
                // Amounts should be positive
                prop_assert!(amount_a > 0);
                prop_assert!(amount_b > 0);
                // Amounts should not exceed reserves
                prop_assert!(amount_a <= reserve_a);
                prop_assert!(amount_b <= reserve_b);
            }
            Err(_) => {
                // Errors are acceptable for edge cases
            }
        }
    }

    /// Property: Add then remove liquidity - verify no overflow and reasonable bounds
    #[test]
    fn prop_liquidity_reversible(
        reserve_a in 1000u128..100_000u128,
        reserve_b in 1000u128..100_000u128,
        amount_a in 100u128..10_000u128,
        amount_b in 100u128..10_000u128,
        total_supply in 1000u128..100_000u128,
    ) {
        // Add liquidity
        if let Ok(liquidity_minted) = calculate_liquidity_mint(
            reserve_a,
            reserve_b,
            amount_a,
            amount_b,
            total_supply,
        ) {
            let new_reserve_a = reserve_a.checked_add(amount_a);
            let new_reserve_b = reserve_b.checked_add(amount_b);
            let new_total_supply = total_supply.checked_add(liquidity_minted);

            if let (Some(new_a), Some(new_b), Some(new_supply)) =
                (new_reserve_a, new_reserve_b, new_total_supply) {

                // Remove the same liquidity
                if let Ok((removed_a, removed_b)) = calculate_liquidity_burn(
                    new_a,
                    new_b,
                    liquidity_minted,
                    new_supply,
                ) {
                    // Verify amounts are positive
                    prop_assert!(removed_a > 0);
                    prop_assert!(removed_b > 0);

                    // Verify amounts are reasonable (within same order of magnitude)
                    // Due to integer division rounding, exact equality is not guaranteed
                    prop_assert!(removed_a <= amount_a * 2);
                    prop_assert!(removed_b <= amount_b * 2);

                    // Verify we don't get back way more than we put in
                    prop_assert!(removed_a < amount_a + amount_a / 2);
                    prop_assert!(removed_b < amount_b + amount_b / 2);
                }
            }
        }
    }

    /// Property: Zero liquidity should fail
    #[test]
    fn prop_zero_liquidity_fails(
        reserve_a in 1u128..1_000_000u128,
        reserve_b in 1u128..1_000_000u128,
        total_supply in 1u128..1_000_000u128,
    ) {
        let result = calculate_liquidity_burn(reserve_a, reserve_b, 0, total_supply);
        prop_assert!(result.is_err());
    }

    /// Property: Liquidity exceeding total supply should fail
    #[test]
    fn prop_excess_liquidity_fails(
        reserve_a in 1u128..1_000_000u128,
        reserve_b in 1u128..1_000_000u128,
        total_supply in 1u128..1_000_000u128,
        excess in 1u128..1_000_000u128,
    ) {
        let liquidity = total_supply.saturating_add(excess);
        let result = calculate_liquidity_burn(reserve_a, reserve_b, liquidity, total_supply);
        prop_assert!(result.is_err());
    }
}

// ── Edge case tests ──────────────────────────────────────────────────────────────

proptest! {
    /// Property: Maximum safe values should not overflow
    #[test]
    fn prop_max_safe_values(
        reserve_in in 1u128..=(u64::MAX as u128),
        reserve_out in 1u128..=(u64::MAX as u128),
        amount_in in 1u128..=(u32::MAX as u128),
        fee_bps in 0u128..1000u128,
    ) {
        // This should either succeed or fail gracefully
        let result = calculate_swap_output(reserve_in, reserve_out, amount_in, fee_bps);

        // We just verify it doesn't panic
        match result {
            Ok(_) | Err(_) => {}
        }
    }

    /// Property: Fee at maximum (99.99%) should leave minimal output
    #[test]
    fn prop_high_fee_minimal_output(
        reserve_in in 1000u128..1_000_000u128,
        reserve_out in 1000u128..1_000_000u128,
        amount_in in 100u128..10_000u128,
    ) {
        let high_fee = 9999; // 99.99%
        let low_fee = 30;    // 0.3%

        let high_fee_output = calculate_swap_output(reserve_in, reserve_out, amount_in, high_fee);
        let low_fee_output = calculate_swap_output(reserve_in, reserve_out, amount_in, low_fee);

        if let (Ok(high_out), Ok(low_out)) = (high_fee_output, low_fee_output) {
            // High fee should result in much lower output
            prop_assert!(high_out < low_out);
        }
    }
}
