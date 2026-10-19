#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn test_mint_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, TokenWithBugs);
    let client = TokenWithBugsClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &String::from_str(&env, "Token"), &String::from_str(&env, "TKN"));
    
    let to = Address::generate(&env);
    client.mint(&admin, &to, &1000);
    assert_eq!(client.balance(&to), 1000);
}

#[test]
#[should_panic(expected = "not admin")]
fn test_mint_non_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, TokenWithBugs);
    let client = TokenWithBugsClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &String::from_str(&env, "Token"), &String::from_str(&env, "TKN"));
    
    let to = Address::generate(&env);
    let not_admin = Address::generate(&env);
    client.mint(&not_admin, &to, &1000);
}
