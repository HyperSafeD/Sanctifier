extern crate std;

use crate::{VestingContract, VestingContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env,
};

#[test]
fn test_vesting_cliff() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);

    // Register a mock token
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token.mint(&admin, &1000);

    let contract_id = env.register_contract(None, VestingContract);
    let client = VestingContractClient::new(&env, &contract_id);

    let start = 1000;
    let cliff = 500;
    let duration = 1000;
    let total_amount = 1000i128;

    client.init(
        &admin,
        &beneficiary,
        &token_id,
        &start,
        &cliff,
        &duration,
        &total_amount,
        &true,
    );

    // Initial check: 0 vested during cliff
    env.ledger().set_timestamp(start + cliff - 1);
    assert_eq!(client.vested_amount(), 0);
    assert_eq!(client.claimable_amount(), 0);

    // Check after cliff: exactly 50% vested (at 500s mark out of 1000s duration)
    env.ledger().set_timestamp(start + cliff);
    assert_eq!(client.vested_amount(), 500);
    assert_eq!(client.claimable_amount(), 500);
}

#[test]
fn test_vesting_linear_and_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    let token_query = soroban_sdk::token::TokenClient::new(&env, &token_id);
    token.mint(&admin, &1000);

    let contract_id = env.register_contract(None, VestingContract);
    let client = VestingContractClient::new(&env, &contract_id);

    client.init(
        &admin,
        &beneficiary,
        &token_id,
        &0,
        &0,
        &1000,
        &1000,
        &false,
    );

    // 25% vesting
    env.ledger().set_timestamp(250);
    assert_eq!(client.vested_amount(), 250);

    // Beneficiary claims
    client.claim();
    assert_eq!(token_query.balance(&beneficiary), 250);
    assert_eq!(client.claimable_amount(), 0);

    // 75% vesting
    env.ledger().set_timestamp(750);
    assert_eq!(client.vested_amount(), 750);
    assert_eq!(client.claimable_amount(), 500); // 750 total - 250 already released

    client.claim();
    assert_eq!(token_query.balance(&beneficiary), 750);
}

#[test]
fn test_vesting_revoke() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    let token_query = soroban_sdk::token::TokenClient::new(&env, &token_id);
    token.mint(&admin, &1000);

    let contract_id = env.register_contract(None, VestingContract);
    let client = VestingContractClient::new(&env, &contract_id);

    client.init(&admin, &beneficiary, &token_id, &0, &0, &1000, &1000, &true);

    // 40% vesting
    env.ledger().set_timestamp(400);
    assert_eq!(client.vested_amount(), 400);

    // Admin revokes
    client.revoke();

    // Admin should get 600 tokens back
    assert_eq!(token_query.balance(&admin), 600);

    // Beneficiary should still be able to claim the 400 tokens that were already vested
    env.ledger().set_timestamp(1000); // Try jumping into the future
    assert_eq!(client.vested_amount(), 400); // Should be capped at revocation timestamp

    client.claim();
    assert_eq!(token_query.balance(&beneficiary), 400);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")] // NoVestedTokens
fn test_claim_nothing_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token.mint(&admin, &1000);

    let contract_id = env.register_contract(None, VestingContract);
    let client = VestingContractClient::new(&env, &contract_id);

    client.init(
        &admin,
        &beneficiary,
        &token_id,
        &100,
        &0,
        &1000,
        &1000,
        &false,
    );

    env.ledger().set_timestamp(50); // Before start
    client.claim();
}

// ── Property-based tests ─────────────────────────────────────────────────────

fn vested_linear(amount: i128, elapsed: u64, duration: u64) -> i128 {
    if elapsed >= duration {
        return amount;
    }
    amount * elapsed as i128 / duration as i128
}

proptest::proptest! {
    #[test]
    fn prop_vested_never_exceeds_total(
        amount in 1i128..1_000_000_000i128,
        elapsed in 0u64..10_000u64,
        duration in 1u64..10_000u64,
    ) {
        let v = vested_linear(amount, elapsed, duration);
        proptest::prop_assert!(v >= 0 && v <= amount);
    }

    #[test]
    fn prop_vested_is_monotonic(
        amount in 1i128..1_000_000_000i128,
        t1 in 0u64..5_000u64,
        t2 in 0u64..5_000u64,
        duration in 1u64..10_000u64,
    ) {
        let (earlier, later) = (t1.min(t2), t1.max(t2));
        proptest::prop_assert!(
            vested_linear(amount, later, duration) >= vested_linear(amount, earlier, duration)
        );
    }

    #[test]
    fn prop_fully_vested_at_duration(
        amount in 1i128..1_000_000_000i128,
        duration in 1u64..10_000u64,
    ) {
        proptest::prop_assert_eq!(vested_linear(amount, duration, duration), amount);
    }

    #[test]
    fn prop_nothing_vested_before_cliff(
        amount in 1i128..1_000_000_000i128,
        cliff in 1u64..5_000u64,
        duration in 1u64..10_000u64,
    ) {
        proptest::prop_assert_eq!(vested_linear(amount, 0, duration.max(cliff)), 0);
    }
}
