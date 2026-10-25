//! Integration tests for the `approve → transfer_from` TOCTOU race described in
//! `docs/allowance-race.md`.
//!
//! Unlike the unit tests in `src/lib.rs`, these drive `transfer_from` against a
//! real Stellar Asset Contract, so the race is proven by token balances rather
//! than by inspecting the allowance slot alone.

use allowance_race::{AllowanceRaceContract, AllowanceRaceContractClient};
use soroban_sdk::{testutils::Address as _, token, Address, Env};

const ALICE_FUNDS: i128 = 1_000;

struct Setup<'a> {
    contract: AllowanceRaceContractClient<'a>,
    token: token::Client<'a>,
    alice: Address,
    bob: Address,
    recipient: Address,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    // `transfer_from` calls `token.transfer(owner, ..)` from inside the contract,
    // so the owner's auth is a non-root invocation.
    env.mock_all_auths_allowing_non_root_auth();

    let token_admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token = token::Client::new(&env, &token_addr);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_addr);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let recipient = Address::generate(&env);
    token_admin_client.mint(&alice, &ALICE_FUNDS);

    let contract_id = env.register_contract(None, AllowanceRaceContract);
    let contract = AllowanceRaceContractClient::new(&env, &contract_id);

    Setup {
        contract,
        token,
        alice,
        bob,
        recipient,
    }
}

/// Reproduces the attack sequence from `docs/allowance-race.md` step by step.
///
/// Alice means to lower Bob's allowance from 100 to 50, but Bob front-runs the
/// second `approve` and ends up moving 150 tokens.
#[test]
fn approve_race_lets_spender_drain_more_than_intended() {
    let s = setup();
    let token_addr = s.token.address.clone();

    // 1. Alice calls approve(Bob, 100).
    s.contract.approve(&s.alice, &s.bob, &100);
    assert_eq!(s.contract.allowance(&s.alice, &s.bob), 100);

    // 2. Alice submits approve(Bob, 50), still pending.
    // 3. Bob front-runs it and spends the full 100.
    s.contract
        .transfer_from(&s.bob, &s.alice, &s.recipient, &token_addr, &100);
    assert_eq!(s.contract.allowance(&s.alice, &s.bob), 0);

    // 4. Alice's approve(Bob, 50) lands. `approve` overwrites without checking
    //    what was spent in between: the time-of-check is Alice reading 100,
    //    the time-of-use is this write.
    s.contract.approve(&s.alice, &s.bob, &50);
    assert_eq!(s.contract.allowance(&s.alice, &s.bob), 50);

    // 5. Bob spends the fresh 50.
    s.contract
        .transfer_from(&s.bob, &s.alice, &s.recipient, &token_addr, &50);

    // Total drained: 150, but Alice intended a cap of 50.
    assert_eq!(s.token.balance(&s.recipient), 150);
    assert_eq!(s.token.balance(&s.alice), ALICE_FUNDS - 150);
    assert_eq!(s.contract.allowance(&s.alice, &s.bob), 0);
}

/// Same front-run, but Alice lowers the allowance with `decrease_allowance`.
/// The delta applies to whatever is left after Bob's spend, so the 100 Alice
/// originally granted is the most Bob can take.
#[test]
fn decrease_allowance_caps_spend_at_original_grant() {
    let s = setup();
    let token_addr = s.token.address.clone();

    s.contract.increase_allowance(&s.alice, &s.bob, &100);

    // Bob front-runs Alice's decrease and spends the full 100.
    s.contract
        .transfer_from(&s.bob, &s.alice, &s.recipient, &token_addr, &100);

    // Alice's decrease_allowance(50) lands: 0 - 50 clamps to 0, no fresh grant.
    s.contract.decrease_allowance(&s.alice, &s.bob, &50);
    assert_eq!(s.contract.allowance(&s.alice, &s.bob), 0);

    // Bob's second spend is rejected.
    let second = s
        .contract
        .try_transfer_from(&s.bob, &s.alice, &s.recipient, &token_addr, &50);
    assert!(second.is_err(), "spender must not get a second allowance");

    assert_eq!(s.token.balance(&s.recipient), 100);
    assert_eq!(s.token.balance(&s.alice), ALICE_FUNDS - 100);
}

/// Without a front-run, `decrease_allowance` lands first and Bob is left with
/// exactly the 50 Alice intended.
#[test]
fn decrease_allowance_without_front_run_leaves_intended_amount() {
    let s = setup();
    let token_addr = s.token.address.clone();

    s.contract.increase_allowance(&s.alice, &s.bob, &100);
    s.contract.decrease_allowance(&s.alice, &s.bob, &50);
    assert_eq!(s.contract.allowance(&s.alice, &s.bob), 50);

    let over = s
        .contract
        .try_transfer_from(&s.bob, &s.alice, &s.recipient, &token_addr, &51);
    assert!(over.is_err(), "spend above the allowance must be rejected");

    s.contract
        .transfer_from(&s.bob, &s.alice, &s.recipient, &token_addr, &50);
    assert_eq!(s.token.balance(&s.recipient), 50);
    assert_eq!(s.contract.allowance(&s.alice, &s.bob), 0);
}
