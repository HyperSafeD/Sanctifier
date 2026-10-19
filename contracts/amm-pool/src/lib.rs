//! # AMM Pool Contract
//!
//! Constant-product AMM (x·y = k) for two tokens on Soroban.
//!
//! ## Public Interface (ABI)
//!
//! | Function | Description |
//! |---|---|
//! | [`AmmPool::add_liquidity`] | Deposit token pair, receive LP tokens |
//! | [`AmmPool::remove_liquidity`] | Burn LP tokens, receive proportional reserves |
//! | [`AmmPool::swap`] | Swap one token for the other |
//! | [`AmmPool::get_price`] | Query the current spot price (scaled by 1 000 000) |
//!
//! ## Error Codes
//!
//! See [`AmmError`] for the full list of contract error variants.
#![no_std]
#![allow(unexpected_cfgs)]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

const MINIMUM_LIQUIDITY: u128 = 1_000;
const PRICE_SCALE: u128 = 1_000_000;

/// Current storage schema version. Increment when storage layout changes and
/// bump the Cargo.toml minor version alongside a migration entry.
pub const CONTRACT_VERSION: u32 = 1;

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
enum DataKey {
    TokenA,
    TokenB,
    ReserveA,
    ReserveB,
    TotalSupply,
    /// Stores the on-chain schema version written by the first add_liquidity
    /// call or an explicit migrate(). Absent on pre-versioning deployments,
    /// which are treated as version 1.
    Version,
}

/// Errors returned by the AMM pool contract.
#[contracterror]
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum AmmError {
    /// One or both deposit amounts are zero.
    ZeroAmount = 1,
    /// `token_a` and `token_b` must be different addresses.
    IdenticalTokens = 2,
    /// The supplied token pair does not match the initialised pair.
    InvalidPair = 3,
    /// Pool has not been seeded with initial liquidity yet.
    PoolNotInitialized = 4,
    /// Reserves are too low to satisfy the requested output.
    InsufficientLiquidity = 5,
    /// Computed output is below the caller-supplied `min_out` guard.
    SlippageExceeded = 6,
    /// Initial mint would produce fewer LP tokens than `MINIMUM_LIQUIDITY`.
    MintBelowMinimum = 7,
    /// Cannot burn the permanently-locked minimum liquidity.
    LockedLiquidity = 8,
    /// A checked arithmetic operation overflowed.
    ArithmeticOverflow = 9,
}

/// Constant-product AMM pool.
#[contract]
pub struct AmmPool;

#[contractimpl]
impl AmmPool {
    /// Deposit `amount_a` of `token_a` and `amount_b` of `token_b` into the pool.
    ///
    /// On the first call the pair is initialised and `MINIMUM_LIQUIDITY` LP tokens
    /// are permanently locked.  Subsequent calls mint LP tokens proportional to the
    /// smaller of the two deposit ratios.
    ///
    /// Returns the number of LP tokens minted to the caller, or `0` on any
    /// validation failure (zero amounts, wrong pair, slippage, overflow).
    pub fn add_liquidity(
        env: Env,
        token_a: Address,
        token_b: Address,
        amount_a: u128,
        amount_b: u128,
        min_lp: u128,
    ) -> u128 {
        env.current_contract_address().require_auth();

        if amount_a == 0 || amount_b == 0 {
            return 0;
        }
        if token_a == token_b {
            return 0;
        }

        let total_supply = read_total_supply(&env).unwrap_or(0);

        if total_supply == 0 {
            let Some(initial_liquidity) = calculate_initial_liquidity(amount_a, amount_b) else {
                return 0;
            };
            let Some(minted) = initial_liquidity.checked_sub(MINIMUM_LIQUIDITY) else {
                return 0;
            };
            if minted < min_lp {
                return 0;
            }

            write_pair(&env, token_a, token_b);
            write_pool_state(&env, amount_a, amount_b, initial_liquidity);
            env.storage()
                .instance()
                .set(&DataKey::Version, &CONTRACT_VERSION);
            return minted;
        }

        let Some((stored_a, stored_b)) = read_pair(&env) else {
            return 0;
        };
        if stored_a != token_a || stored_b != token_b {
            return 0;
        }

        let Some(reserve_a) = read_reserve_a(&env) else {
            return 0;
        };
        let Some(reserve_b) = read_reserve_b(&env) else {
            return 0;
        };
        let Some(minted) =
            calculate_liquidity_mint(reserve_a, reserve_b, amount_a, amount_b, total_supply)
        else {
            return 0;
        };
        if minted < min_lp {
            return 0;
        }

        let Some(next_reserve_a) = reserve_a.checked_add(amount_a) else {
            return 0;
        };
        let Some(next_reserve_b) = reserve_b.checked_add(amount_b) else {
            return 0;
        };
        let Some(next_total_supply) = total_supply.checked_add(minted) else {
            return 0;
        };

        write_pool_state(&env, next_reserve_a, next_reserve_b, next_total_supply);
        minted
    }

    /// Burn `lp_amount` LP tokens and withdraw proportional reserves.
    ///
    /// `min_a` / `min_b` are slippage guards; the call returns `(0, 0)` if
    /// either computed amount falls below the respective minimum.
    ///
    /// The permanently-locked `MINIMUM_LIQUIDITY` can never be burned.
    pub fn remove_liquidity(env: Env, lp_amount: u128, min_a: u128, min_b: u128) -> (u128, u128) {
        env.current_contract_address().require_auth();

        if lp_amount == 0 {
            return (0, 0);
        }

        let Some(total_supply) = read_total_supply(&env) else {
            return (0, 0);
        };
        let Some(remaining_supply) = total_supply.checked_sub(lp_amount) else {
            return (0, 0);
        };
        if remaining_supply < MINIMUM_LIQUIDITY {
            return (0, 0);
        }

        let Some(reserve_a) = read_reserve_a(&env) else {
            return (0, 0);
        };
        let Some(reserve_b) = read_reserve_b(&env) else {
            return (0, 0);
        };

        let Some(amount_a) = proportional_amount(reserve_a, lp_amount, total_supply) else {
            return (0, 0);
        };
        let Some(amount_b) = proportional_amount(reserve_b, lp_amount, total_supply) else {
            return (0, 0);
        };

        if amount_a < min_a || amount_b < min_b {
            return (0, 0);
        }
        if amount_a == 0 || amount_b == 0 {
            return (0, 0);
        }

        let Some(next_reserve_a) = reserve_a.checked_sub(amount_a) else {
            return (0, 0);
        };
        let Some(next_reserve_b) = reserve_b.checked_sub(amount_b) else {
            return (0, 0);
        };

        write_pool_state(&env, next_reserve_a, next_reserve_b, remaining_supply);
        (amount_a, amount_b)
    }

    /// Swap `amount_in` of `token_in` for the other token in the pair.
    ///
    /// `min_out` is a slippage guard; returns `0` if the computed output is
    /// below it.  Uses the constant-product formula without a fee.
    pub fn swap(env: Env, token_in: Address, amount_in: u128, min_out: u128) -> u128 {
        env.current_contract_address().require_auth();

        if amount_in == 0 {
            return 0;
        }

        let Some(reserve_a) = read_reserve_a(&env) else {
            return 0;
        };
        let Some(reserve_b) = read_reserve_b(&env) else {
            return 0;
        };
        let Some((token_a, token_b)) = read_pair(&env) else {
            return 0;
        };
        let amount_out = if token_in == token_a {
            execute_swap(reserve_a, reserve_b, amount_in, min_out)
        } else if token_in == token_b {
            execute_swap(reserve_b, reserve_a, amount_in, min_out)
        } else {
            return 0;
        };

        if token_in == token_a {
            let Some(next_reserve_a) = reserve_a.checked_add(amount_in) else {
                return 0;
            };
            let Some(next_reserve_b) = reserve_b.checked_sub(amount_out) else {
                return 0;
            };
            let Some(total_supply) = read_total_supply(&env) else {
                return 0;
            };
            write_pool_state(&env, next_reserve_a, next_reserve_b, total_supply);
            return amount_out;
        }

        let Some(next_reserve_b) = reserve_b.checked_add(amount_in) else {
            return 0;
        };
        let Some(next_reserve_a) = reserve_a.checked_sub(amount_out) else {
            return 0;
        };
        let Some(total_supply) = read_total_supply(&env) else {
            return 0;
        };
        write_pool_state(&env, next_reserve_a, next_reserve_b, total_supply);
        amount_out
    }

    /// Returns the on-chain schema version. Pre-versioning pools without a
    /// stored key are implicitly at version 1.
    pub fn get_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get::<DataKey, u32>(&DataKey::Version)
            .unwrap_or(CONTRACT_VERSION)
    }

    /// Advance the schema version from `from_version` to `from_version + 1`.
    /// Returns `true` on success and `false` when the stored version does not
    /// match `from_version` (prevents replaying an already-applied migration).
    pub fn migrate(env: Env, from_version: u32) -> bool {
        env.current_contract_address().require_auth();
        let current: u32 = env
            .storage()
            .instance()
            .get::<DataKey, u32>(&DataKey::Version)
            .unwrap_or(CONTRACT_VERSION);
        if current != from_version {
            return false;
        }
        env.storage()
            .instance()
            .set(&DataKey::Version, &current.saturating_add(1));
        true
    }

    /// Return the spot price of `token_in` denominated in `token_out`, scaled
    /// by `PRICE_SCALE` (1 000 000).  Returns `0` for an unknown pair or an
    /// uninitialised pool.
    pub fn get_price(env: Env, token_in: Address, token_out: Address) -> u128 {
        let Some(reserve_a) = read_reserve_a(&env) else {
            return 0;
        };
        let Some(reserve_b) = read_reserve_b(&env) else {
            return 0;
        };
        let Some((token_a, token_b)) = read_pair(&env) else {
            return 0;
        };

        if token_in == token_a && token_out == token_b {
            return scaled_ratio(reserve_b, reserve_a).unwrap_or(0);
        }
        if token_in == token_b && token_out == token_a {
            return scaled_ratio(reserve_a, reserve_b).unwrap_or(0);
        }

        0
    }
}

fn execute_swap(reserve_in: u128, reserve_out: u128, amount_in: u128, min_out: u128) -> u128 {
    if reserve_in == 0 || reserve_out == 0 {
        return 0;
    }

    let Some(amount_out) = calculate_swap_output(reserve_in, reserve_out, amount_in) else {
        return 0;
    };
    if amount_out < min_out {
        return 0;
    }

    if amount_out == 0 || amount_out >= reserve_out {
        return 0;
    }

    amount_out
}

fn calculate_initial_liquidity(amount_a: u128, amount_b: u128) -> Option<u128> {
    let product = amount_a.checked_mul(amount_b)?;
    integer_sqrt(product)
}

pub fn calculate_liquidity_mint(
    reserve_a: u128,
    reserve_b: u128,
    amount_a: u128,
    amount_b: u128,
    total_supply: u128,
) -> Option<u128> {
    if reserve_a == 0 || reserve_b == 0 || total_supply == 0 {
        return None;
    }

    let liquidity_a = amount_a.checked_mul(total_supply)?.checked_div(reserve_a)?;
    let liquidity_b = amount_b.checked_mul(total_supply)?.checked_div(reserve_b)?;

    let minted = min_u128(liquidity_a, liquidity_b);
    if minted == 0 {
        return None;
    }
    Some(minted)
}

pub fn calculate_swap_output(reserve_in: u128, reserve_out: u128, amount_in: u128) -> Option<u128> {
    let numerator = amount_in.checked_mul(reserve_out)?;
    let denominator = reserve_in.checked_add(amount_in)?;
    numerator.checked_div(denominator)
}

fn proportional_amount(reserve: u128, lp_amount: u128, total_supply: u128) -> Option<u128> {
    let numerator = reserve.checked_mul(lp_amount)?;
    numerator.checked_div(total_supply)
}

fn scaled_ratio(numerator: u128, denominator: u128) -> Option<u128> {
    if numerator == 0 || denominator == 0 {
        return None;
    }

    let scaled = numerator.checked_mul(PRICE_SCALE)?;
    scaled.checked_div(denominator)
}

fn integer_sqrt(value: u128) -> Option<u128> {
    if value == 0 {
        return Some(0);
    }

    let mut estimate = value;
    let mut next = ceil_half(value)?;

    while next < estimate {
        estimate = next;
        let quotient = value.checked_div(estimate)?;
        let sum = estimate.checked_add(quotient)?;
        next = ceil_half(sum)?;
    }

    Some(estimate)
}

fn ceil_half(value: u128) -> Option<u128> {
    let incremented = value.checked_add(1)?;
    incremented.checked_div(2)
}

fn write_pair(env: &Env, token_a: Address, token_b: Address) {
    env.storage().instance().set(&DataKey::TokenA, &token_a);
    env.storage().instance().set(&DataKey::TokenB, &token_b);
}

fn read_pair(env: &Env) -> Option<(Address, Address)> {
    let token_a = read_address(env, DataKey::TokenA)?;
    let token_b = read_address(env, DataKey::TokenB)?;
    Some((token_a, token_b))
}

fn write_pool_state(env: &Env, reserve_a: u128, reserve_b: u128, total_supply: u128) {
    env.storage().instance().set(&DataKey::ReserveA, &reserve_a);
    env.storage().instance().set(&DataKey::ReserveB, &reserve_b);
    env.storage()
        .instance()
        .set(&DataKey::TotalSupply, &total_supply);
}

fn read_address(env: &Env, key: DataKey) -> Option<Address> {
    env.storage().instance().get::<DataKey, Address>(&key)
}

fn read_reserve_a(env: &Env) -> Option<u128> {
    read_u128(env, DataKey::ReserveA)
}

fn read_reserve_b(env: &Env) -> Option<u128> {
    read_u128(env, DataKey::ReserveB)
}

fn read_total_supply(env: &Env) -> Option<u128> {
    env.storage()
        .instance()
        .get::<DataKey, u128>(&DataKey::TotalSupply)
}

fn read_u128(env: &Env, key: DataKey) -> Option<u128> {
    env.storage().instance().get::<DataKey, u128>(&key)
}

fn min_u128(left: u128, right: u128) -> u128 {
    if left < right {
        left
    } else {
        right
    }
}
