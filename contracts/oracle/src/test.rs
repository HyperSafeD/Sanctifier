extern crate std;

use crate::{HeartbeatEvent, OracleContract, OracleContractClient};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    vec, Address, Env, IntoVal,
};

fn setup_oracle(env: &Env) -> (OracleContractClient, Vec<Address>) {
    let contract_id = env.register_contract(None, OracleContract);
    let client = OracleContractClient::new(env, &contract_id);

    let validator1 = Address::generate(env);
    let validator2 = Address::generate(env);
    let validator3 = Address::generate(env);

    let validators = vec![env, validator1.clone(), validator2.clone(), validator3.clone()];
    let threshold = 2u32;

    client.init(&validators, &threshold);

    (client, validators)
}

#[test]
fn test_heartbeat_no_data() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _validators) = setup_oracle(&env);

    // Set initial timestamp
    env.ledger().set_timestamp(1000);

    // Call heartbeat when no price data exists
    client.heartbeat();

    // Verify heartbeat event was emitted
    let events = env.events().all();
    let last_event = events.last().unwrap();

    assert_eq!(
        last_event.topics.first().unwrap(),
        soroban_sdk::symbol_short!("heartbeat").into_val(&env)
    );

    // Event data should show no last update
    let heartbeat_event: HeartbeatEvent = last_event.data.clone().try_into_val(&env).unwrap();
    assert_eq!(heartbeat_event.last_update, 0);
    assert_eq!(heartbeat_event.current_time, 1000);
    assert_eq!(heartbeat_event.time_since_update, 1000);
}

#[test]
fn test_heartbeat_with_recent_data() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, validators) = setup_oracle(&env);

    // Update price at timestamp 1000
    env.ledger().set_timestamp(1000);
    let signers = vec![&env, validators.get(0).unwrap(), validators.get(1).unwrap()];
    client.update_price(&100, &1000, &signers);

    // Call heartbeat at timestamp 1300
    env.ledger().set_timestamp(1300);
    client.heartbeat();

    // Verify heartbeat event
    let events = env.events().all();
    let last_event = events.last().unwrap();

    let heartbeat_event: HeartbeatEvent = last_event.data.clone().try_into_val(&env).unwrap();
    assert_eq!(heartbeat_event.last_update, 1000);
    assert_eq!(heartbeat_event.current_time, 1300);
    assert_eq!(heartbeat_event.time_since_update, 300);
}

#[test]
fn test_heartbeat_with_stale_data() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, validators) = setup_oracle(&env);

    // Update price at timestamp 1000
    env.ledger().set_timestamp(1000);
    let signers = vec![&env, validators.get(0).unwrap(), validators.get(1).unwrap()];
    client.update_price(&100, &1000, &signers);

    // Call heartbeat much later at timestamp 10000 (9000 seconds elapsed)
    env.ledger().set_timestamp(10000);
    client.heartbeat();

    // Verify heartbeat shows significant time elapsed
    let events = env.events().all();
    let last_event = events.last().unwrap();

    let heartbeat_event: HeartbeatEvent = last_event.data.clone().try_into_val(&env).unwrap();
    assert_eq!(heartbeat_event.last_update, 1000);
    assert_eq!(heartbeat_event.current_time, 10000);
    assert_eq!(heartbeat_event.time_since_update, 9000);
}

#[test]
fn test_is_alive_no_data() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _validators) = setup_oracle(&env);

    env.ledger().set_timestamp(1000);

    // Should return false when no price data exists
    assert_eq!(client.is_alive(&3600), false);
}

#[test]
fn test_is_alive_with_fresh_data() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, validators) = setup_oracle(&env);

    // Update price at timestamp 1000
    env.ledger().set_timestamp(1000);
    let signers = vec![&env, validators.get(0).unwrap(), validators.get(1).unwrap()];
    client.update_price(&100, &1000, &signers);

    // Check at timestamp 1500 (500 seconds later)
    env.ledger().set_timestamp(1500);

    // Should be alive with max_age of 3600 seconds
    assert_eq!(client.is_alive(&3600), true);

    // Should be alive with max_age of 500 seconds (exactly at boundary)
    assert_eq!(client.is_alive(&500), true);

    // Should NOT be alive with max_age of 499 seconds
    assert_eq!(client.is_alive(&499), false);
}

#[test]
fn test_is_alive_with_stale_data() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, validators) = setup_oracle(&env);

    // Update price at timestamp 1000
    env.ledger().set_timestamp(1000);
    let signers = vec![&env, validators.get(0).unwrap(), validators.get(1).unwrap()];
    client.update_price(&100, &1000, &signers);

    // Check at timestamp 5000 (4000 seconds later)
    env.ledger().set_timestamp(5000);

    // Should NOT be alive with max_age of 3600 seconds
    assert_eq!(client.is_alive(&3600), false);

    // Should be alive with max_age of 4000 seconds (exactly at boundary)
    assert_eq!(client.is_alive(&4000), true);

    // Should be alive with max_age of 5000 seconds
    assert_eq!(client.is_alive(&5000), true);
}

#[test]
fn test_is_alive_allows_non_reverting_checks() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, validators) = setup_oracle(&env);

    // Update price at timestamp 1000
    env.ledger().set_timestamp(1000);
    let signers = vec![&env, validators.get(0).unwrap(), validators.get(1).unwrap()];
    client.update_price(&100, &1000, &signers);

    // Move to timestamp where data is stale
    env.ledger().set_timestamp(5000);

    // is_alive returns false without reverting
    assert_eq!(client.is_alive(&100), false);

    // Can safely check before calling get_price
    if client.is_alive(&100) {
        // This branch won't execute
        assert!(false, "Should not reach here");
    } else {
        // Safe to handle the stale case
        assert!(true);
    }
}

#[test]
fn test_monitoring_workflow() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, validators) = setup_oracle(&env);

    // Initial state - oracle initialized but no price data
    env.ledger().set_timestamp(1000);
    assert_eq!(client.is_alive(&3600), false);

    // Price feed comes online
    let signers = vec![&env, validators.get(0).unwrap(), validators.get(1).unwrap()];
    client.update_price(&100, &1000, &signers);

    // Monitoring check 1: 10 seconds later
    env.ledger().set_timestamp(1010);
    assert_eq!(client.is_alive(&60), true);
    client.heartbeat();

    // Monitoring check 2: 30 seconds after first update
    env.ledger().set_timestamp(1030);
    assert_eq!(client.is_alive(&60), true);
    client.heartbeat();

    // New price update
    env.ledger().set_timestamp(1050);
    client.update_price(&105, &1050, &signers);

    // Monitoring check 3: Feeder still alive
    env.ledger().set_timestamp(1080);
    assert_eq!(client.is_alive(&60), true);
    client.heartbeat();

    // Monitoring check 4: Feeder goes offline (120 seconds with no update)
    env.ledger().set_timestamp(1200);
    assert_eq!(client.is_alive(&60), false);
    client.heartbeat();

    // Verify last heartbeat shows staleness
    let events = env.events().all();
    let last_event = events.last().unwrap();
    let heartbeat_event: HeartbeatEvent = last_event.data.clone().try_into_val(&env).unwrap();
    assert_eq!(heartbeat_event.time_since_update, 150); // 1200 - 1050
}
