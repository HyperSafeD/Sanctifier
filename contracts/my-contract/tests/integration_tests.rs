#![cfg(test)]
#![allow(unexpected_cfgs)]

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String,
};

use my_contract::{Token, TokenClient, TokenError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup(env: &Env) -> (TokenClient, Address) {
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

fn setup_with_balance(env: &Env, holder: &Address, amount: i128) -> TokenClient {
    let (client, admin) = setup(env);
    client.mint(&admin, holder, &amount);
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
    let (client, admin) = setup(&env);
    let alice = Address::generate(&env);

    client.mint(&admin, &alice, &1_000i128);
    assert_eq!(client.balance(&alice), 1_000);
}

#[test]
fn mint_accumulates_across_calls() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let alice = Address::generate(&env);

    client.mint(&admin, &alice, &500i128);
    client.mint(&admin, &alice, &300i128);
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

    // Ledger sequence defaults to 0; expiration_ledger=1000 is well in the future.
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

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn transfer_entire_balance_leaves_zero() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 1_000);

    client.transfer(&alice, &bob, &1_000i128);

    assert_eq!(client.balance(&alice), 0);
    assert_eq!(client.balance(&bob), 1_000);
}

#[test]
fn allowance_returns_zero_after_expiry() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (client, _) = setup(&env);

    // Approve with expiration_ledger=5.
    client.approve(&alice, &bob, &500i128, &5u32);

    // Advance ledger past expiry.
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
    assert_eq!(result, Err(Ok(TokenError::AlreadyInitialized)));
}

#[test]
fn transfer_from_consumes_exact_allowance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 1_000);

    client.approve(&alice, &bob, &100i128, &1_000u32);
    client.transfer_from(&bob, &alice, &carol, &100i128);

    // Allowance should now be zero; a second draw must fail.
    let result = client.try_transfer_from(&bob, &alice, &carol, &1i128);
    assert!(result.is_err());
}

#[test]
fn burn_from_reduces_allowance_independently_of_balance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 500);

    client.approve(&alice, &bob, &300i128, &1_000u32);
    client.burn_from(&bob, &alice, &100i128);

    assert_eq!(client.balance(&alice), 400);
    assert_eq!(client.allowance(&alice, &bob), 200);
}

#[test]
fn balance_of_unknown_address_is_zero() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let stranger = Address::generate(&env);

    assert_eq!(client.balance(&stranger), 0);
}

#[test]
fn approve_overwrites_previous_allowance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (client, _) = setup(&env);

    client.approve(&alice, &bob, &500i128, &1_000u32);
    client.approve(&alice, &bob, &100i128, &2_000u32);

    assert_eq!(client.allowance(&alice, &bob), 100);
}

// ---------------------------------------------------------------------------
// Unauthorised call attempts
// ---------------------------------------------------------------------------

#[test]
fn transfer_requires_from_auth() {
    let env = Env::default();
    // No mock_all_auths — require_auth() will abort without a matching mock.
    let id = env.register_contract(None, Token);
    let client = TokenClient::new(&env, &id);
    let from = Address::generate(&env);
    let to = Address::generate(&env);

    let result = client.try_transfer(&from, &to, &100i128);
    assert!(result.is_err());
}

#[test]
fn burn_requires_from_auth() {
    let env = Env::default();
    let id = env.register_contract(None, Token);
    let client = TokenClient::new(&env, &id);
    let from = Address::generate(&env);

    let result = client.try_burn(&from, &100i128);
    assert!(result.is_err());
}

#[test]
fn approve_requires_from_auth() {
    let env = Env::default();
    let id = env.register_contract(None, Token);
    let client = TokenClient::new(&env, &id);
    let from = Address::generate(&env);
    let spender = Address::generate(&env);

    let result = client.try_approve(&from, &spender, &500i128, &1_000u32);
    assert!(result.is_err());
}

#[test]
fn transfer_from_requires_spender_auth() {
    let env = Env::default();
    let id = env.register_contract(None, Token);
    let client = TokenClient::new(&env, &id);
    let spender = Address::generate(&env);
    let from = Address::generate(&env);
    let to = Address::generate(&env);

    let result = client.try_transfer_from(&spender, &from, &to, &100i128);
    assert!(result.is_err());
}

#[test]
fn burn_from_requires_spender_auth() {
    let env = Env::default();
    let id = env.register_contract(None, Token);
    let client = TokenClient::new(&env, &id);
    let spender = Address::generate(&env);
    let from = Address::generate(&env);

    let result = client.try_burn_from(&spender, &from, &100i128);
    assert!(result.is_err());
}

#[test]
fn mint_fails_when_not_initialized() {
    let env = Env::default();
    let id = env.register_contract(None, Token);
    let client = TokenClient::new(&env, &id);
    let to = Address::generate(&env);

    let result = client.try_mint(&to, &100i128);
    assert_eq!(result, Err(Ok(TokenError::NotInitialized)));
}

#[test]
fn transfer_fails_with_insufficient_balance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 50);

    let result = client.try_transfer(&alice, &bob, &100i128);
    assert_eq!(result, Err(Ok(TokenError::InsufficientBalance)));
}

#[test]
fn burn_fails_with_insufficient_balance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 50);

    let result = client.try_burn(&alice, &100i128);
    assert_eq!(result, Err(Ok(TokenError::InsufficientBalance)));
}

#[test]
fn transfer_from_fails_with_insufficient_allowance() {
    let env = Env::default();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);
    let client = setup_with_balance(&env, &alice, 1_000);

    client.approve(&alice, &bob, &50i128, &1_000u32);

    let result = client.try_transfer_from(&bob, &alice, &carol, &100i128);
    assert_eq!(result, Err(Ok(TokenError::InsufficientAllowance)));
}
