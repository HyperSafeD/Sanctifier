#![cfg(test)]
//! Property-based / fuzz tests for the SEP-41 token contract.
//!
//! Each test uses bolero's `check!()` harness to drive arbitrary inputs through
//! the token's public entry points and assert that no undefined behaviour or
//! unexpected panics occur.  All arithmetic in the contract uses checked
//! operations, so only the well-typed error variants are ever returned.

use bolero::{check, generator::*};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

use my_contract::{Token, TokenClient};

fn fresh_client(env: &Env) -> (TokenClient, Address) {
    let admin = Address::generate(env);
    let id = env.register_contract(None, Token);
    let client = TokenClient::new(env, &id);
    env.mock_all_auths();
    client.initialize(
        &admin,
        &7u32,
        &String::from_str(env, "Fuzz Token"),
        &String::from_str(env, "FUZZ"),
    );
    (client, admin)
}

/// Any non-negative `amount` passed to `mint` must either succeed or return a
/// well-typed error — never an unstructured panic.
#[test]
fn fuzz_mint_no_panic() {
    check!()
        .with_generator(gen::<i128>().with().bounds(0i128..=i128::MAX))
        .for_each(|amount| {
            let env = Env::default();
            let (client, admin) = fresh_client(&env);
            let to = Address::generate(&env);
            let _ = client.try_mint(&admin, &to, amount);
        });
}

/// Any combination of `(amount_a, amount_b)` passed through mint → transfer
/// must leave balances consistent: sender decreases, receiver increases, sum
/// is conserved.
#[test]
fn fuzz_transfer_balance_conservation() {
    check!()
        .with_generator(gen::<(u32, u32)>())
        .for_each(|(mint_amount, transfer_amount)| {
            let env = Env::default();
            let (client, admin) = fresh_client(&env);
            let alice = Address::generate(&env);
            let bob = Address::generate(&env);

            let mint_amt = *mint_amount as i128;
            let transfer_amt = *transfer_amount as i128;

            let _ = client.try_mint(&admin, &alice, &mint_amt);
            let balance_before = client.balance(&alice);

            if let Ok(Ok(())) = client.try_transfer(&alice, &bob, &transfer_amt) {
                let alice_after = client.balance(&alice);
                let bob_after = client.balance(&bob);
                // Conservation: alice lost exactly transfer_amt, bob gained it.
                assert_eq!(balance_before - transfer_amt, alice_after);
                assert_eq!(bob_after, transfer_amt);
            }
        });
}

/// Approve followed by transfer_from must never let the spender withdraw more
/// than approved, and the allowance must never go negative.
#[test]
fn fuzz_allowance_monotone_decrease() {
    check!()
        .with_generator(gen::<(u32, u32)>())
        .for_each(|(approve_amt, draw_amt)| {
            let env = Env::default();
            let (client, admin) = fresh_client(&env);
            let alice = Address::generate(&env);
            let bob = Address::generate(&env);
            let carol = Address::generate(&env);

            let approve = *approve_amt as i128;
            let draw = *draw_amt as i128;

            let _ = client.try_mint(&admin, &alice, &approve);
            let _ = client.try_approve(&alice, &bob, &approve, &1_000u32);

            let allowance_before = client.allowance(&alice, &bob);
            if let Ok(Ok(())) = client.try_transfer_from(&bob, &alice, &carol, &draw) {
                let allowance_after = client.allowance(&alice, &bob);
                assert!(allowance_after >= 0, "allowance went negative");
                assert_eq!(allowance_before - draw, allowance_after);
            }
        });
}

/// Burn must never produce a negative balance.
#[test]
fn fuzz_burn_balance_never_negative() {
    check!()
        .with_generator(gen::<(u32, u32)>())
        .for_each(|(mint_amt, burn_amt)| {
            let env = Env::default();
            let (client, admin) = fresh_client(&env);
            let alice = Address::generate(&env);

            let _ = client.try_mint(&admin, &alice, &(*mint_amt as i128));

            if let Ok(Ok(())) = client.try_burn(&alice, &(*burn_amt as i128)) {
                assert!(
                    client.balance(&alice) >= 0,
                    "balance went negative after burn"
                );
            }
        });
}
