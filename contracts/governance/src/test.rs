use crate::{GovernorContract, GovernorContractClient, ProposalState};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger as _},
    vec, Address, Env, IntoVal, Symbol, Val, Vec,
};

#[soroban_sdk::contract]
pub struct VotingToken;

#[soroban_sdk::contractimpl]
impl VotingToken {
    pub fn balance(e: Env, addr: Address) -> i128 {
        e.storage().instance().get(&addr).unwrap_or(0i128)
    }
    pub fn set_balance(e: Env, addr: Address, amount: i128) {
        e.storage().instance().set(&addr, &amount);
    }
    pub fn total_supply(_e: Env) -> i128 {
        10_000
    }
}

#[soroban_sdk::contract]
pub struct MockTimelock;

#[soroban_sdk::contractimpl]
impl MockTimelock {
    pub fn schedule(
        _e: Env,
        _proposer: Address,
        _target: Address,
        _fn: Symbol,
        _args: Vec<Val>,
        _salt: soroban_sdk::BytesN<32>,
        _delay: u64,
    ) {
    }
    pub fn execute(
        _e: Env,
        _proposer: Address,
        _target: Address,
        _fn: Symbol,
        _args: Vec<Val>,
        _salt: soroban_sdk::BytesN<32>,
    ) {
    }
    pub fn get_min_delay(_e: Env) -> u64 {
        3600
    }
}

#[test]
fn test_governance_full_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);

    // 1. Setup Mock Token and Timelock
    let token_id = env.register_contract(None, VotingToken);
    let token_client = VotingTokenClient::new(&env, &token_id);
    token_client.set_balance(&proposer, &1000);
    token_client.set_balance(&voter1, &6000);
    token_client.set_balance(&voter2, &3000);

    let timelock_id = env.register_contract(None, MockTimelock);

    // 2. Setup Governor
    let governor_id = env.register_contract(None, GovernorContract);
    let client = GovernorContractClient::new(&env, &governor_id);

    client.init(
        &token_id,
        &timelock_id,
        &4000,  // 40% quorum
        &5001,  // >50% majority
        &86400, // 1 day period
        &3600,  // 1 hour delay
        &500,   // min 500 tokens to propose
    );

    // 3. Propose
    let target = Address::generate(&env);
    let function = symbol_short!("test");
    let args: Vec<Val> = vec![&env, 42u32.into_val(&env)];
    let description = symbol_short!("prop1");

    let proposal_id = client.propose(
        &proposer,
        &vec![&env, target.clone()],
        &vec![&env, function.clone()],
        &vec![&env, args.clone()],
        &description,
    );

    assert_eq!(client.state(&proposal_id), ProposalState::Pending);

    // 4. Wait for delay
    env.ledger().set_timestamp(3601);
    assert_eq!(client.state(&proposal_id), ProposalState::Active);

    // 5. Vote
    client.cast_vote(&voter1, &proposal_id, &1); // Support (6000 votes)
    client.cast_vote(&voter2, &proposal_id, &0); // Against (3000 votes)

    // Total votes: 9000 (90%) -> Quorum Met. Majority: 6000/9000 (66%) -> Threshold Met.

    // 6. End voting period
    env.ledger().set_timestamp(3601 + 86401);
    assert_eq!(client.state(&proposal_id), ProposalState::Succeeded);

    // 7. Queue and Execute
    client.queue(&proposal_id);
    assert_eq!(client.state(&proposal_id), ProposalState::Queued);

    client.execute(&proposal_id);
    assert_eq!(client.state(&proposal_id), ProposalState::Executed);
}

#[test]
fn test_quorum_not_met() {
    let env = Env::default();
    env.mock_all_auths();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);

    let token_id = env.register_contract(None, VotingToken);
    let token_client = VotingTokenClient::new(&env, &token_id);
    token_client.set_balance(&proposer, &1000);
    token_client.set_balance(&voter1, &2000);

    let timelock_id = env.register_contract(None, MockTimelock);

    let governor_id = env.register_contract(None, GovernorContract);
    let client = GovernorContractClient::new(&env, &governor_id);

    client.init(&token_id, &timelock_id, &4000, &5001, &1000, &0, &500);

    let proposal_id = client.propose(
        &proposer,
        &vec![&env],
        &vec![&env],
        &vec![&env],
        &symbol_short!("prop"),
    );
    client.cast_vote(&voter1, &proposal_id, &1);

    env.ledger().set_timestamp(1001);
    assert_eq!(client.state(&proposal_id), ProposalState::Defeated);
}
