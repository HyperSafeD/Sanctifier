use crate::{Token, TokenClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup(env: &Env) -> (TokenClient<'_>, Address) {
    let admin = Address::generate(env);
    let id = env.register_contract(None, Token);
    let client = TokenClient::new(env, &id);
    env.mock_all_auths();
    client.initialize(
        &admin,
        &7u32,
        &String::from_str(env, "Test Token"),
        &String::from_str(env, "TEST"),
    );
    (client, admin)
}

fn setup_with_balance<'a>(env: &'a Env, holder: &Address, amount: i128) -> TokenClient<'a> {
    let (client, _admin) = setup(env);
    client.mint(holder, &amount);
    client
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn initialize_stores_metadata() {
    let env = Env::default();
    let (client, _) = setup(&env);

    assert_eq!(client.decimals(), 7);
    assert_eq!(client.name(), String::from_str(&env, "Test Token"));
    assert_eq!(client.symbol(), String::from_str(&env, "TEST"));
}

#[test]
fn mint_increases_balance() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let alice = Address::generate(&env);

    client.mint(&alice, &1_000i128);
    assert_eq!(client.balance(&alice), 1_000);
}

#[test]
fn mint_accumulates_across_calls() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let alice = Address::generate(&env);

    client.mint(&alice, &500i128);
    client.mint(&alice, &300i128);
    assert_eq!(client.balance(&alice), 800);
}

#[test]
fn transfer_moves_funds() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 1_000);

    client.transfer(&alice, &bob, &400i128);

    assert_eq!(client.balance(&alice), 600);
    assert_eq!(client.balance(&bob), 400);
}

#[test]
fn approve_sets_allowance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (client, _) = setup(&env);

    client.approve(&alice, &bob, &500i128, &1_000u32);

    assert_eq!(client.allowance(&alice, &bob), 500);
}

#[test]
fn transfer_from_spends_allowance_and_moves_funds() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 1_000);

    client.approve(&alice, &bob, &300i128, &1_000u32);
    client.transfer_from(&bob, &alice, &carol, &200i128);

    assert_eq!(client.balance(&alice), 800);
    assert_eq!(client.balance(&carol), 200);
    assert_eq!(client.allowance(&alice, &bob), 100);
}

#[test]
fn burn_reduces_balance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 1_000);

    client.burn(&alice, &400i128);

    assert_eq!(client.balance(&alice), 600);
}

#[test]
fn burn_from_uses_allowance_and_reduces_balance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 1_000);

    client.approve(&alice, &bob, &300i128, &1_000u32);
    client.burn_from(&bob, &alice, &200i128);

    assert_eq!(client.balance(&alice), 800);
    assert_eq!(client.allowance(&alice, &bob), 100);
}

#[test]
fn allowance_returns_zero_after_expiry() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (client, _) = setup(&env);

    client.approve(&alice, &bob, &500i128, &5u32);

    env.ledger().with_mut(|l| l.sequence_number = 6);

    assert_eq!(client.allowance(&alice, &bob), 0);
}

#[test]
fn transfer_from_fails_on_expired_allowance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 1_000);

    client.approve(&alice, &bob, &500i128, &5u32);
    env.ledger().with_mut(|l| l.sequence_number = 6);

    let result = client.try_transfer_from(&bob, &alice, &carol, &100i128);
    assert!(result.is_err());
}

#[test]
fn double_initialize_fails() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let result = client.try_initialize(
        &admin,
        &7u32,
        &String::from_str(&env, "Test Token"),
        &String::from_str(&env, "TEST"),
    );
    assert!(result.is_err());
}

#[test]
fn mint_fails_when_not_initialized() {
    let env = Env::default();
    let id = env.register_contract(None, Token);
    let client = TokenClient::new(&env, &id);
    let to = Address::generate(&env);

    let result = client.try_mint(&to, &100i128);
    assert!(result.is_err());
}

#[test]
fn transfer_fails_with_insufficient_balance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 50);

    let result = client.try_transfer(&alice, &bob, &100i128);
    assert!(result.is_err());
}
