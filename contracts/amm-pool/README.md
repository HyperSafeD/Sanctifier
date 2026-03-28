# AMM Pool with Property-Based Testing

This contract implements an Automated Market Maker (AMM) liquidity pool using the constant product formula (x \* y = k), similar to Uniswap V2.

## Features

- **Swap calculations** with configurable fees
- **Add liquidity** with automatic LP token minting
- **Remove liquidity** with proportional token returns
- **Overflow/underflow protection** using checked arithmetic
- **Comprehensive property-based testing** using proptest

## Core Functions

### `calculate_swap_output`

Calculates the output amount for a token swap using the constant product formula with fees.

```rust
pub fn calculate_swap_output(
    reserve_in: u128,
    reserve_out: u128,
    amount_in: u128,
    fee_bps: u128,
) -> Result<u128, &'static str>
```

### `calculate_liquidity_mint`

Calculates LP tokens to mint when adding liquidity to the pool.

```rust
pub fn calculate_liquidity_mint(
    reserve_a: u128,
    reserve_b: u128,
    amount_a: u128,
    amount_b: u128,
    total_supply: u128,
) -> Result<u128, &'static str>
```

### `calculate_liquidity_burn`

Calculates token amounts to return when burning LP tokens.

```rust
pub fn calculate_liquidity_burn(
    reserve_a: u128,
    reserve_b: u128,
    liquidity: u128,
    total_supply: u128,
) -> Result<(u128, u128), &'static str>
```

## Property-Based Testing

The contract includes extensive property-based tests using `proptest` to verify:

### Swap Properties

- ✅ No overflow in swap calculations
- ✅ Constant product formula preservation (k increases due to fees)
- ✅ Monotonicity (larger inputs yield larger outputs)
- ✅ Zero amounts and reserves are rejected
- ✅ Output never exceeds reserves

### Liquidity Properties

- ✅ No overflow in liquidity calculations
- ✅ Proportionality of LP tokens to deposits
- ✅ Reversibility (add then remove returns similar amounts)
- ✅ Amounts never exceed reserves
- ✅ Zero liquidity and excess liquidity are rejected

### Edge Cases

- ✅ Maximum safe values handled gracefully
- ✅ High fees result in minimal output
- ✅ All operations fail gracefully with errors (no panics)

## Running Tests

### Unit Tests

```bash
cd contracts/amm-pool
cargo test
```

### Property-Based Tests

```bash
cd contracts/amm-pool
cargo test --test proptest_amm
```

### Run with more test cases

```bash
PROPTEST_CASES=10000 cargo test --test proptest_amm
```

### Run specific property test

```bash
cargo test --test proptest_amm prop_swap_no_overflow
```

## Security Considerations

1. **Checked Arithmetic**: All operations use `checked_*` methods to prevent overflow/underflow
2. **Input Validation**: All inputs are validated before processing
3. **Graceful Failures**: Functions return `Result` types instead of panicking
4. **Fee Bounds**: Fees are capped at 100% (10000 basis points)
5. **Reserve Protection**: Outputs never exceed available reserves

## Mathematical Invariants

The property tests verify these key invariants:

1. **Constant Product**: `x * y ≥ k` (increases due to fees)
2. **Conservation**: Tokens are never created or destroyed incorrectly
3. **Proportionality**: LP tokens represent proportional ownership
4. **Monotonicity**: More input always yields more output
5. **Reversibility**: Add/remove liquidity operations are approximately reversible

## Example Usage

```rust
use amm_pool::*;

// Swap 100 tokens with 0.3% fee
let output = calculate_swap_output(
    1000,  // reserve_in
    2000,  // reserve_out
    100,   // amount_in
    30,    // fee_bps (0.3%)
).unwrap();

// Add initial liquidity
let lp_tokens = calculate_liquidity_mint(
    0,     // reserve_a (empty pool)
    0,     // reserve_b (empty pool)
    1000,  // amount_a
    2000,  // amount_b
    0,     // total_supply
).unwrap();

// Remove liquidity
let (amount_a, amount_b) = calculate_liquidity_burn(
    1000,  // reserve_a
    2000,  // reserve_b
    100,   // liquidity to burn
    500,   // total_supply
).unwrap();
```

## Contributing

When adding new functionality:

1. Add corresponding property-based tests
2. Verify all arithmetic uses checked operations
3. Ensure functions return `Result` types
4. Run the full test suite with high iteration counts
