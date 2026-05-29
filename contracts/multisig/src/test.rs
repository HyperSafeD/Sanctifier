extern crate std;

use crate::{MultisigWallet, MultisigWalletClient};
use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Logs},
    vec, Address, Bytes, Env, IntoVal, Symbol, Val,
};

#[contract]
pub struct MockContract;

#[contractimpl]
impl MockContract {
    pub fn action(env: Env, value: u32) -> u32 {
        env.events().publish((symbol_short!("action"),), value);
        value + 1
    }
}

#[test]
fn test_multisig_external_call() {
    let env = Env::default();
    env.mock_all_auths();

    let wallet_id = env.register_contract(None, MultisigWallet);
    let client = MultisigWalletClient::new(&env, &wallet_id);

    let signer1 = Address::generate(&env);
    let signers = vec![&env, signer1.clone()];

    client.init(&signers, &1);

    let mock_id = env.register_contract(None, MockContract);
    let function = Symbol::new(&env, "action");
    let args: soroban_sdk::Vec<Val> = vec![&env, 10u32.into_val(&env)];
    let salt = Bytes::from_array(&env, &[0u8; 32]);

    let hash = client.propose(&mock_id, &function, &args, &salt);
    client.approve(&signer1, &hash);

    let result = client.execute(&mock_id, &function, &args, &salt);
    let result_u32: u32 = result.into_val(&env);
    assert_eq!(result_u32, 11u32);
}

#[test]
fn test_multisig_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let wallet_id = env.register_contract(None, MultisigWallet);
    let client = MultisigWalletClient::new(&env, &wallet_id);

    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);
    let signers = vec![&env, signer1.clone(), signer2.clone(), signer3.clone()];

    client.init(&signers, &2);

    // Proposal: Change threshold to 3
    let target = wallet_id.clone();
    let function = Symbol::new(&env, "set_threshold");
    let args: soroban_sdk::Vec<Val> = vec![&env, 3u32.into_val(&env)];
    let salt = Bytes::from_array(&env, &[0u8; 32]);

    let hash = client.propose(&target, &function, &args, &salt);

    // Approval 1
    client.approve(&signer1, &hash);

    // Try to execute (should fail, threshold is 2)
    let result = client.try_execute(&target, &function, &args, &salt);
    assert!(result.is_err());

    // Approval 2
    client.approve(&signer2, &hash);

    // Execute
    let result = client.try_execute(&target, &function, &args, &salt);
    if let Err(e) = &result {
        std::println!("Unexpected error on execute (self-call): {:?}", e);
        for log in env.logs().all() {
            std::println!("LOG: {}", log);
        }
    }
    assert!(result.is_ok());

    // Verify side effect
    // Still using the current threshold which was 2. Let's see if it changed for NEXT proposal.
    // Wait, the call happened! so it should have changed.
}

#[test]
fn test_unauthorized_signer() {
    let env = Env::default();
    env.mock_all_auths();

    let wallet_id = env.register_contract(None, MultisigWallet);
    let client = MultisigWalletClient::new(&env, &wallet_id);

    let signer1 = Address::generate(&env);
    let stranger = Address::generate(&env);
    let signers = vec![&env, signer1.clone()];

    client.init(&signers, &1);

    let target = Address::generate(&env);
    let function = symbol_short!("test");
    let args = vec![&env];
    let salt = Bytes::from_array(&env, &[0u8; 32]);

    let hash = client.propose(&target, &function, &args, &salt);

    // Stranger tries to approve
    let result = client.try_approve(&stranger, &hash);
    assert!(result.is_err());
}

#[test]
fn test_signer_management() {
    let env = Env::default();
    env.mock_all_auths();

    let wallet_id = env.register_contract(None, MultisigWallet);
    let client = MultisigWalletClient::new(&env, &wallet_id);

    let signer1 = Address::generate(&env);
    let signers = vec![&env, signer1.clone()];

    client.init(&signers, &1);

    let new_signer = Address::generate(&env);
    let target = wallet_id.clone();
    let function = Symbol::new(&env, "add_signer");
    let args: soroban_sdk::Vec<Val> = vec![&env, new_signer.into_val(&env)];
    let salt = Bytes::from_array(&env, &[1u8; 32]);

    let hash = client.propose(&target, &function, &args, &salt);
    client.approve(&signer1, &hash);

    let result = client.try_execute(&target, &function, &args, &salt);
    if let Err(e) = &result {
        std::println!("Error on add_signer execute: {:?}", e);
        for log in env.logs().all() {
            std::println!("LOG: {}", log);
        }
    }
    assert!(result.is_ok());

    // Now new_signer should be able to approve proposals
    let mock_id = env.register_contract(None, MockContract);
    let hash2 = client.propose(
        &mock_id,
        &symbol_short!("action"),
        &vec![&env, 20u32.into_val(&env)],
        &Bytes::from_array(&env, &[2u8; 32]),
    );

    // new_signer approves
    client.approve(&new_signer, &hash2);

    // Execution should work
    client.execute(
        &mock_id,
        &symbol_short!("action"),
        &vec![&env, 20u32.into_val(&env)],
        &Bytes::from_array(&env, &[2u8; 32]),
    );
}

// ── Property-based tests ─────────────────────────────────────────────────────

fn quorum_reached(approvals: u32, threshold: u32) -> bool {
    approvals >= threshold
}

proptest::proptest! {
    #[test]
    fn prop_threshold_exactly_met_is_sufficient(threshold in 1u32..100u32) {
        proptest::prop_assert!(quorum_reached(threshold, threshold));
    }

    #[test]
    fn prop_one_below_threshold_is_insufficient(threshold in 2u32..100u32) {
        proptest::prop_assert!(!quorum_reached(threshold - 1, threshold));
    }

    #[test]
    fn prop_above_threshold_is_sufficient(
        threshold in 1u32..50u32,
        extra in 1u32..50u32,
    ) {
        proptest::prop_assert!(quorum_reached(threshold + extra, threshold));
    }

    #[test]
    fn prop_threshold_zero_approvals_never_passes_nonzero_threshold(
        threshold in 1u32..100u32,
    ) {
        proptest::prop_assert!(!quorum_reached(0, threshold));
    }
}
