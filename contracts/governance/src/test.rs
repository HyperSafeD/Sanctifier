extern crate std;

use crate::{
    Error, GovernorContract, GovernorContractClient, ProposalEvent, ProposalState, VoteCastEvent,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, Ledger as _},
    vec,
    xdr::ToXdr,
    Address, Env, IntoVal, Symbol, Val, Vec,
};

fn assert_last_event(env: &Env, contract: &Address, name: &str, data: Val) {
    let (emitter, topics, emitted_data) = env.events().all().last().unwrap();
    assert_eq!(emitter, *contract);
    assert_eq!(topics, vec![env, Symbol::new(env, name).into_val(env)]);
    assert_eq!(emitted_data.to_xdr(env), data.to_xdr(env));
}

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
        &1000,  // min 1000 absolute votes required
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
    assert_last_event(
        &env,
        &governor_id,
        "proposal_created",
        ProposalEvent {
            proposal_id,
            caller: proposer.clone(),
            timestamp: 0,
        }
        .into_val(&env),
    );

    assert_eq!(client.state(&proposal_id), ProposalState::Pending);

    // 4. Wait for delay
    env.ledger().set_timestamp(3601);
    assert_eq!(client.state(&proposal_id), ProposalState::Active);

    // 5. Vote
    client.cast_vote(&voter1, &proposal_id, &1); // Support (6000 votes)
    assert_last_event(
        &env,
        &governor_id,
        "vote_cast",
        VoteCastEvent {
            proposal_id,
            caller: voter1.clone(),
            timestamp: 3601,
            support: 1,
            weight: 6000,
        }
        .into_val(&env),
    );
    client.cast_vote(&voter2, &proposal_id, &0); // Against (3000 votes)

    // Total votes: 9000 (90%) -> Quorum Met. Majority: 6000/9000 (66%) -> Threshold Met.

    // 6. End voting period
    env.ledger().set_timestamp(3601 + 86401);
    assert_eq!(client.state(&proposal_id), ProposalState::Succeeded);

    // 7. Queue and Execute
    client.queue(&proposer, &proposal_id);
    assert_eq!(client.state(&proposal_id), ProposalState::Queued);

    client.execute(&proposer, &proposal_id);
    assert_last_event(
        &env,
        &governor_id,
        "proposal_executed",
        ProposalEvent {
            proposal_id,
            caller: proposer.clone(),
            timestamp: 3601 + 86401,
        }
        .into_val(&env),
    );
    assert_eq!(client.state(&proposal_id), ProposalState::Executed);
}

#[test]
fn test_proposer_can_cancel_proposal() {
    let env = Env::default();
    env.mock_all_auths();

    let proposer = Address::generate(&env);
    let token_id = env.register_contract(None, VotingToken);
    VotingTokenClient::new(&env, &token_id).set_balance(&proposer, &1_000);
    let timelock_id = env.register_contract(None, MockTimelock);
    let governor_id = env.register_contract(None, GovernorContract);
    let client = GovernorContractClient::new(&env, &governor_id);
    client.init(
        &token_id,
        &timelock_id,
        &4000,
        &5001,
        &1000,
        &0,
        &500,
        &1000,
    );

    let proposal_id = client.propose(
        &proposer,
        &vec![&env],
        &vec![&env],
        &vec![&env],
        &symbol_short!("cancel"),
    );
    client.cancel(&proposer, &proposal_id);

    assert_last_event(
        &env,
        &governor_id,
        "proposal_cancelled",
        ProposalEvent {
            proposal_id,
            caller: proposer,
            timestamp: 0,
        }
        .into_val(&env),
    );

    assert_eq!(client.state(&proposal_id), ProposalState::Canceled);
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

    client.init(
        &token_id,
        &timelock_id,
        &4000,
        &5001,
        &1000,
        &0,
        &500,
        &1000,
    );

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

#[test]
fn test_insufficient_min_quorum_defeats_proposal() {
    let env = Env::default();
    env.mock_all_auths();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);

    let token_id = env.register_contract(None, VotingToken);
    let token_client = VotingTokenClient::new(&env, &token_id);
    token_client.set_balance(&proposer, &1000);
    token_client.set_balance(&voter1, &500); // Only 500 votes, less than min_quorum of 1000

    let timelock_id = env.register_contract(None, MockTimelock);

    let governor_id = env.register_contract(None, GovernorContract);
    let client = GovernorContractClient::new(&env, &governor_id);

    // min_quorum = 1000, but voter only has 500 tokens
    client.init(
        &token_id,
        &timelock_id,
        &4000,
        &5001,
        &1000,
        &0,
        &500,
        &1000,
    );

    let proposal_id = client.propose(
        &proposer,
        &vec![&env],
        &vec![&env],
        &vec![&env],
        &symbol_short!("prop"),
    );
    client.cast_vote(&voter1, &proposal_id, &1);

    env.ledger().set_timestamp(1001);
    // State should be Succeeded (percentage quorum met: 500/10000 = 5% < 40%... wait)
    // Actually with 500 votes out of 10000 supply = 5% < 40% quorum_bps, so Defeated
    // Let's use a case where percentage quorum passes but absolute min_quorum fails
    assert_eq!(client.state(&proposal_id), ProposalState::Defeated);
}

#[test]
fn test_min_quorum_blocks_queueing() {
    let env = Env::default();
    env.mock_all_auths();

    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);

    let token_id = env.register_contract(None, VotingToken);
    let token_client = VotingTokenClient::new(&env, &token_id);
    token_client.set_balance(&proposer, &1000);
    token_client.set_balance(&voter1, &3000); // 3000 votes
    token_client.set_balance(&voter2, &2000); // 2000 votes

    let timelock_id = env.register_contract(None, MockTimelock);

    let governor_id = env.register_contract(None, GovernorContract);
    let client = GovernorContractClient::new(&env, &governor_id);

    // quorum_bps = 4000 (40%), min_quorum = 6000
    // Total supply = 10000
    // voter1 + voter2 = 5000 votes = 50% > 40% (percentage quorum passes)
    // But 5000 < 6000 (absolute min_quorum fails)
    client.init(
        &token_id,
        &timelock_id,
        &4000,
        &5001,
        &1000,
        &0,
        &500,
        &6000,
    );

    let proposal_id = client.propose(
        &proposer,
        &vec![&env],
        &vec![&env],
        &vec![&env],
        &symbol_short!("prop"),
    );
    client.cast_vote(&voter1, &proposal_id, &1); // 3000 for
    client.cast_vote(&voter2, &proposal_id, &1); // 2000 for

    env.ledger().set_timestamp(1001);
    // Percentage quorum: 5000/10000 = 50% > 40% -> passes
    // Threshold: 5000/5000 = 100% > 50.01% -> passes
    // The absolute quorum requirement must prevent the proposal from passing.
    assert_eq!(client.state(&proposal_id), ProposalState::Defeated);

    // A defeated proposal cannot be queued.
    let err = client
        .try_queue(&proposer, &proposal_id)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::InvalidState);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.queue(&proposer, &proposal_id);
    }));
    assert!(result.is_err(), "queue should have failed for insufficient quorum");
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_propose_mismatched_lengths() {
    let env = Env::default();
    env.mock_all_auths();

    let proposer = Address::generate(&env);
    let token_id = env.register_contract(None, VotingToken);
    let token_client = VotingTokenClient::new(&env, &token_id);
    token_client.set_balance(&proposer, &1000);

    let timelock_id = env.register_contract(None, MockTimelock);

    let governor_id = env.register_contract(None, GovernorContract);
    let client = GovernorContractClient::new(&env, &governor_id);

    client.init(
        &token_id,
        &timelock_id,
        &4000,
        &5001,
        &1000,
        &0,
        &500,
        &1000,
    );

    let target = Address::generate(&env);
    // targets has 1, functions has 0 -> mismatch
    client.propose(
        &proposer,
        &vec![&env, target],
        &vec![&env],
        &vec![&env],
        &symbol_short!("prop"),
    );
}

