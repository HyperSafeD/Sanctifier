#![cfg(test)]
#![allow(unexpected_cfgs)]

use amm_pool::AmmPool;
use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env};

#[contract]
pub struct AmmPoolHarness;

#[contractimpl]
impl AmmPoolHarness {
    pub fn add_liquidity(
        env: Env,
        token_a: Address,
        token_b: Address,
        amount_a: u128,
        amount_b: u128,
        min_lp: u128,
    ) -> u128 {
        AmmPool::add_liquidity(env, token_a, token_b, amount_a, amount_b, min_lp)
    }

    pub fn remove_liquidity(env: Env, lp_amount: u128, min_a: u128, min_b: u128) -> (u128, u128) {
        AmmPool::remove_liquidity(env, lp_amount, min_a, min_b)
    }

    pub fn swap(env: Env, token_in: Address, amount_in: u128, min_out: u128) -> u128 {
        AmmPool::swap(env, token_in, amount_in, min_out)
    }

    pub fn get_price(env: Env, token_in: Address, token_out: Address) -> u128 {
        AmmPool::get_price(env, token_in, token_out)
    }
}

#[test]
fn add_liquidity_initializes_pool_and_locks_minimum_supply() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    let minted = client.add_liquidity(&token_a, &token_b, &4_000u128, &9_000u128, &5_000u128);

    assert_eq!(minted, 5_000);
    assert_eq!(client.get_price(&token_a, &token_b), 2_250_000);
}

#[test]
fn add_liquidity_rejects_initial_deposit_below_locked_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    assert_eq!(
        client.add_liquidity(&token_a, &token_b, &1_000u128, &1_000u128, &1u128),
        0
    );
}

#[test]
fn add_liquidity_rejects_zero_amounts() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    assert_eq!(
        client.add_liquidity(&token_a, &token_b, &0u128, &2_000u128, &1u128),
        0
    );
}

#[test]
fn add_liquidity_mints_proportional_lp_tokens() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    let _ = client.add_liquidity(&token_a, &token_b, &4_000u128, &9_000u128, &5_000u128);
    let minted = client.add_liquidity(&token_a, &token_b, &2_000u128, &4_500u128, &2_900u128);

    assert_eq!(minted, 3_000);
}

#[test]
fn remove_liquidity_returns_proportional_reserves() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    let _ = client.add_liquidity(&token_a, &token_b, &4_000u128, &9_000u128, &5_000u128);
    let amounts = client.remove_liquidity(&2_000u128, &1_333u128, &3_000u128);

    assert_eq!(amounts, (1_333, 3_000));
}

#[test]
fn remove_liquidity_cannot_burn_locked_supply() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    let _ = client.add_liquidity(&token_a, &token_b, &4_000u128, &9_000u128, &5_000u128);
    assert_eq!(client.remove_liquidity(&6_000u128, &1u128, &1u128), (0, 0));
}

#[test]
fn swap_token_a_for_token_b_updates_price_and_reserves() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    let _ = client.add_liquidity(&token_a, &token_b, &4_000u128, &9_000u128, &5_000u128);
    let output = client.swap(&token_a, &1_000u128, &1_800u128);

    assert_eq!(output, 1_800);
    assert_eq!(client.get_price(&token_a, &token_b), 1_440_000);
}

#[test]
fn swap_token_b_for_token_a_works_in_reverse_direction() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    let _ = client.add_liquidity(&token_a, &token_b, &4_000u128, &9_000u128, &5_000u128);
    let output = client.swap(&token_b, &900u128, &360u128);

    assert_eq!(output, 363);
}

#[test]
fn swap_enforces_max_slippage() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    let _ = client.add_liquidity(&token_a, &token_b, &4_000u128, &9_000u128, &5_000u128);
    assert_eq!(client.swap(&token_a, &1_000u128, &1_900u128), 0);
}

#[test]
fn swap_rejects_zero_liquidity_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_in = Address::generate(&env);

    assert_eq!(client.swap(&token_in, &100u128, &1u128), 0);
}

#[test]
fn get_price_uses_integer_arithmetic_only() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    let _ = client.add_liquidity(&token_a, &token_b, &7_000u128, &3_500u128, &3_900u128);

    assert_eq!(client.get_price(&token_a, &token_b), 500_000);
    assert_eq!(client.get_price(&token_b, &token_a), 2_000_000);
}

#[test]
fn get_price_rejects_unknown_pair() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &contract_id);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);
    let token_c = Address::generate(&env);

    let _ = client.add_liquidity(&token_a, &token_b, &4_000u128, &9_000u128, &5_000u128);
    assert_eq!(client.get_price(&token_a, &token_c), 0);
}

// ---------------------------------------------------------------------------
// ABI surface stability tests
//
// These tests compile-check that the public interface of AmmPool has not
// changed in a breaking way.  They do NOT need to run on-chain; the mere
// fact that they compile is the assertion.  If a function signature changes
// the test file will fail to compile, making the breakage visible in CI.
// ---------------------------------------------------------------------------

/// Verify that `add_liquidity` accepts the documented parameter types and
/// returns `u128`.
#[test]
fn abi_add_liquidity_signature_is_stable() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &id);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    // Signature: (Env, Address, Address, u128, u128, u128) -> u128
    let _: u128 = client.add_liquidity(&a, &b, &4_000u128, &9_000u128, &5_000u128);
}

/// Verify that `remove_liquidity` accepts the documented parameter types and
/// returns `(u128, u128)`.
#[test]
fn abi_remove_liquidity_signature_is_stable() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &id);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let _ = client.add_liquidity(&a, &b, &4_000u128, &9_000u128, &5_000u128);
    // Signature: (Env, u128, u128, u128) -> (u128, u128)
    let _: (u128, u128) = client.remove_liquidity(&2_000u128, &0u128, &0u128);
}

/// Verify that `swap` accepts the documented parameter types and returns `u128`.
#[test]
fn abi_swap_signature_is_stable() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &id);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let _ = client.add_liquidity(&a, &b, &4_000u128, &9_000u128, &5_000u128);
    // Signature: (Env, Address, u128, u128) -> u128
    let _: u128 = client.swap(&a, &500u128, &0u128);
}

/// Verify that `get_price` accepts the documented parameter types and returns
/// `u128`.
#[test]
fn abi_get_price_signature_is_stable() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, AmmPoolHarness);
    let client = AmmPoolHarnessClient::new(&env, &id);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let _ = client.add_liquidity(&a, &b, &4_000u128, &9_000u128, &5_000u128);
    // Signature: (Env, Address, Address) -> u128
    let _: u128 = client.get_price(&a, &b);
}
