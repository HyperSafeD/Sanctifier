//! Contract versioning convention tests for AmmPool.
//!
//! Fixtures
//! --------
//! * `fixture_empty_pool`     – registered contract, no liquidity, no version key yet
//! * `fixture_v1_funded_pool` – pool with initial liquidity; version key stamped as 1
//! * `fixture_v2_pool`        – pool that has been migrated from v1 → v2

#![cfg(test)]
#![allow(unexpected_cfgs)]

use amm_pool::{AmmPool, CONTRACT_VERSION};
use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env};

// ── Harness ──────────────────────────────────────────────────────────────────

#[contract]
pub struct AmmVersionHarness;

#[contractimpl]
impl AmmVersionHarness {
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

    pub fn get_version(env: Env) -> u32 {
        AmmPool::get_version(env)
    }

    pub fn migrate(env: Env, from_version: u32) -> bool {
        AmmPool::migrate(env, from_version)
    }
}

// ── Fixtures ─────────────────────────────────────────────────────────────────

fn fixture_empty_pool(env: &Env) -> AmmVersionHarnessClient<'_> {
    let id = env.register_contract(None, AmmVersionHarness);
    AmmVersionHarnessClient::new(env, &id)
}

fn fixture_v1_funded_pool(env: &Env) -> (AmmVersionHarnessClient<'_>, Address, Address) {
    env.mock_all_auths();
    let client = fixture_empty_pool(env);
    let token_a = Address::generate(env);
    let token_b = Address::generate(env);
    client.add_liquidity(&token_a, &token_b, &4_000u128, &9_000u128, &5_000u128);
    (client, token_a, token_b)
}

fn fixture_v2_pool(env: &Env) -> (AmmVersionHarnessClient<'_>, Address, Address) {
    env.mock_all_auths();
    let (client, ta, tb) = fixture_v1_funded_pool(env);
    let migrated = client.migrate(&1u32);
    assert!(migrated, "fixture_v2_pool: migration from v1 must succeed");
    (client, ta, tb)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[test]
fn version_constant_is_one() {
    assert_eq!(CONTRACT_VERSION, 1);
}

#[test]
fn fresh_pool_without_liquidity_reports_implicit_version() {
    let env = Env::default();
    let client = fixture_empty_pool(&env);
    // No liquidity means no Version key; falls back to CONTRACT_VERSION.
    assert_eq!(client.get_version(), CONTRACT_VERSION);
}

#[test]
fn version_key_is_stamped_on_first_add_liquidity() {
    let env = Env::default();
    let (client, _, _) = fixture_v1_funded_pool(&env);
    assert_eq!(client.get_version(), 1);
}

#[test]
fn get_version_matches_contract_version_constant() {
    let env = Env::default();
    let (client, _, _) = fixture_v1_funded_pool(&env);
    assert_eq!(client.get_version(), CONTRACT_VERSION);
}

#[test]
fn migrate_advances_version_from_one_to_two() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _) = fixture_v1_funded_pool(&env);

    let success = client.migrate(&1u32);

    assert!(success);
    assert_eq!(client.get_version(), 2);
}

#[test]
fn migrate_rejects_wrong_from_version() {
    let env = Env::default();
    let (client, _, _) = fixture_v1_funded_pool(&env);

    let result = client.migrate(&99u32);

    assert!(!result);
    assert_eq!(
        client.get_version(),
        1,
        "version must not change on failed migration"
    );
}

#[test]
fn migrate_cannot_be_replayed_after_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _) = fixture_v1_funded_pool(&env);

    assert!(client.migrate(&1u32));
    let replay = client.migrate(&1u32);

    assert!(
        !replay,
        "replaying an already-applied migration must return false"
    );
    assert_eq!(client.get_version(), 2);
}

#[test]
fn migrate_chain_advances_version_sequentially() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _) = fixture_v1_funded_pool(&env);

    assert!(client.migrate(&1u32));
    assert_eq!(client.get_version(), 2);

    assert!(client.migrate(&2u32));
    assert_eq!(client.get_version(), 3);

    assert!(client.migrate(&3u32));
    assert_eq!(client.get_version(), 4);
}

#[test]
fn version_survives_subsequent_add_liquidity_calls() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, ta, tb) = fixture_v1_funded_pool(&env);

    client.add_liquidity(&ta, &tb, &2_000u128, &4_500u128, &2_900u128);

    assert_eq!(
        client.get_version(),
        1,
        "version must not change across normal operations"
    );
}

#[test]
fn v2_pool_fixture_reports_version_two() {
    let env = Env::default();
    let (client, _, _) = fixture_v2_pool(&env);
    assert_eq!(client.get_version(), 2);
}

#[test]
fn migrate_on_fresh_pool_without_liquidity_uses_implicit_version() {
    let env = Env::default();
    env.mock_all_auths();
    let client = fixture_empty_pool(&env);

    // No explicit Version key: implicit version is CONTRACT_VERSION (1).
    let success = client.migrate(&1u32);

    assert!(success);
    assert_eq!(client.get_version(), 2);
}
