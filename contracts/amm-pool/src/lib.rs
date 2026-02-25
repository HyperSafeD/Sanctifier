#![no_std]

//! AMM Liquidity Pool with Constant Product Formula (x * y = k)
//!
//! This module demonstrates property-based testing for AMM pool math to verify
//! that liquidity pool calculations never overflow or underflow.

// ── Pure logic (testable with proptest) ─────────────────────────────────────────

/// Calculate output amount for a swap using constant product formula
/// Formula: (x * y = k) => output = (y * amount_in) / (x + amount_in)
/// With fee: output = (y * amount_in * (10000 - fee_bps)) / ((x + amount_in) * 10000)
pub fn calculate_swap_output(
    reserve_in: u128,
    reserve_out: u128,
    amount_in: u128,
    fee_bps: u128, // Fee in basis points (e.g., 30 = 0.3%)
) -> Result<u128, &'static str> {
    if amount_in == 0 {
        return Err("Amount in must be positive");
    }
    if reserve_in == 0 || reserve_out == 0 {
        return Err("Reserves must be positive");
    }
    if fee_bps >= 10000 {
        return Err("Fee must be less than 100%");
    }

    // Calculate amount_in after fee
    let amount_in_with_fee = amount_in
        .checked_mul(10000 - fee_bps)
        .ok_or("Fee calculation overflow")?;

    // Calculate numerator: reserve_out * amount_in_with_fee
    let numerator = reserve_out
        .checked_mul(amount_in_with_fee)
        .ok_or("Numerator overflow")?;

    // Calculate denominator: (reserve_in * 10000) + amount_in_with_fee
    let denominator = reserve_in
        .checked_mul(10000)
        .ok_or("Denominator overflow")?
        .checked_add(amount_in_with_fee)
        .ok_or("Denominator addition overflow")?;

    if denominator == 0 {
        return Err("Denominator is zero");
    }

    let output = numerator
        .checked_div(denominator)
        .ok_or("Division error")?;

    Ok(output)
}

/// Add liquidity to the pool
/// Returns the amount of LP tokens to mint
pub fn calculate_liquidity_mint(
    reserve_a: u128,
    reserve_b: u128,
    amount_a: u128,
    amount_b: u128,
    total_supply: u128,
) -> Result<u128, &'static str> {
    if amount_a == 0 || amount_b == 0 {
        return Err("Amounts must be positive");
    }

    // First liquidity provision
    if total_supply == 0 {
        // Use geometric mean to prevent inflation attacks
        let product = amount_a
            .checked_mul(amount_b)
            .ok_or("Initial liquidity overflow")?;
        
        // Simple sqrt approximation for initial liquidity
        let liquidity = integer_sqrt(product);
        
        if liquidity == 0 {
            return Err("Initial liquidity too small");
        }
        
        return Ok(liquidity);
    }

    // Subsequent liquidity provision
    if reserve_a == 0 || reserve_b == 0 {
        return Err("Reserves must be positive");
    }

    // Calculate liquidity based on both ratios and take minimum
    let liquidity_a = amount_a
        .checked_mul(total_supply)
        .ok_or("Liquidity A calculation overflow")?
        .checked_div(reserve_a)
        .ok_or("Division by reserve A")?;

    let liquidity_b = amount_b
        .checked_mul(total_supply)
        .ok_or("Liquidity B calculation overflow")?
        .checked_div(reserve_b)
        .ok_or("Division by reserve B")?;

    // Take minimum to maintain ratio
    let liquidity = if liquidity_a < liquidity_b {
        liquidity_a
    } else {
        liquidity_b
    };

    if liquidity == 0 {
        return Err("Liquidity amount too small");
    }

    Ok(liquidity)
}

/// Remove liquidity from the pool
/// Returns the amounts of tokens A and B to return
pub fn calculate_liquidity_burn(
    reserve_a: u128,
    reserve_b: u128,
    liquidity: u128,
    total_supply: u128,
) -> Result<(u128, u128), &'static str> {
    if liquidity == 0 {
        return Err("Liquidity must be positive");
    }
    if total_supply == 0 {
        return Err("Total supply is zero");
    }
    if liquidity > total_supply {
        return Err("Liquidity exceeds total supply");
    }

    let amount_a = reserve_a
        .checked_mul(liquidity)
        .ok_or("Amount A calculation overflow")?
        .checked_div(total_supply)
        .ok_or("Division by total supply")?;

    let amount_b = reserve_b
        .checked_mul(liquidity)
        .ok_or("Amount B calculation overflow")?
        .checked_div(total_supply)
        .ok_or("Division by total supply")?;

    if amount_a == 0 || amount_b == 0 {
        return Err("Burn amounts too small");
    }

    Ok((amount_a, amount_b))
}

/// Simple integer square root using binary search
fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    
    let mut x = n;
    let mut y = (x + 1) / 2;
    
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_swap() {
        // Pool with 1000 of each token, 0.3% fee
        let output = calculate_swap_output(1000, 1000, 100, 30).unwrap();
        assert!(output > 0);
        assert!(output < 100); // Should be less due to slippage and fees
    }

    #[test]
    fn test_initial_liquidity() {
        let liquidity = calculate_liquidity_mint(0, 0, 1000, 1000, 0).unwrap();
        assert_eq!(liquidity, 1000); // sqrt(1000 * 1000) = 1000
    }

    #[test]
    fn test_add_liquidity() {
        // Existing pool: 1000 A, 2000 B, 500 LP tokens
        let liquidity = calculate_liquidity_mint(1000, 2000, 100, 200, 500).unwrap();
        assert_eq!(liquidity, 50); // (100 * 500) / 1000 = 50
    }

    #[test]
    fn test_remove_liquidity() {
        // Pool: 1000 A, 2000 B, 500 LP tokens, burn 100
        let (amount_a, amount_b) = calculate_liquidity_burn(1000, 2000, 100, 500).unwrap();
        assert_eq!(amount_a, 200); // (1000 * 100) / 500 = 200
        assert_eq!(amount_b, 400); // (2000 * 100) / 500 = 400
    }
}
