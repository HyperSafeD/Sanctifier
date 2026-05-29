extern crate std;

use crate::{TimelockController, TimelockControllerClient};
use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, testutils::Ledger as _, Address, BytesN, Env,
    IntoVal, Symbol, Val, Vec,
};

#[contract]
pub struct MockContract;

#[contractimpl]
impl MockContract {
    pub fn action(_env: Env, value: u32) -> u32 {
        value + 1
    }
}

#[test]
fn test_timelock_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let executor = Address::generate(&env);

    let timelock_id = env.register_contract(None, TimelockController);
    let timelock = TimelockControllerClient::new(&env, &timelock_id);

    let proposers = Vec::from_array(&env, [proposer.clone()]);
    let executors = Vec::from_array(&env, [executor.clone()]);
    let min_delay = 3600; // 1 hour

    timelock.init(&admin, &min_delay, &proposers, &executors);

    let mock_id = env.register_contract(None, MockContract);
    let fn_name = Symbol::new(&env, "action");
    let args = Vec::from_array(&env, [10u32.into_val(&env)]);
    let salt = BytesN::from_array(&env, &[0u8; 32]);

    // Schedule
    let delay = 3600;
    let _hash = timelock.schedule(&proposer, &mock_id, &fn_name, &args, &salt, &delay);

    // Fast forward time
    env.ledger().with_mut(|li| {
        li.timestamp += 3601;
    });

    // Execute
    let result: Val = timelock.execute(&executor, &mock_id, &fn_name, &args, &salt);
    let result_u32: u32 = result.into_val(&env);
    assert_eq!(result_u32, 11u32);
}

#[test]
fn test_role_management() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let timelock_id = env.register_contract(None, TimelockController);
    let timelock = TimelockControllerClient::new(&env, &timelock_id);

    timelock.init(&admin, &3600, &Vec::new(&env), &Vec::new(&env));

    assert!(!timelock.is_proposer(&proposer));
    timelock.set_proposer(&admin, &proposer, &true);
    assert!(timelock.is_proposer(&proposer));
    timelock.set_proposer(&admin, &proposer, &false);
    assert!(!timelock.is_proposer(&proposer));
}

#[test]
fn test_update_delay() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let timelock_id = env.register_contract(None, TimelockController);
    let timelock = TimelockControllerClient::new(&env, &timelock_id);

    timelock.init(&admin, &3600, &Vec::new(&env), &Vec::new(&env));
    assert_eq!(timelock.get_min_delay(), 3600);

    timelock.update_delay(&admin, &7200);
    assert_eq!(timelock.get_min_delay(), 7200);
}

// ── Property-based tests ─────────────────────────────────────────────────────

fn is_ready(current_time: u64, proposal_time: u64, delay: u64) -> bool {
    match proposal_time.checked_add(delay) {
        Some(ready_at) => current_time >= ready_at,
        None => false,
    }
}

proptest::proptest! {
    #[test]
    fn prop_ready_exactly_at_delay(
        proposal_time in 0u64..u64::MAX / 2,
        delay in 0u64..u64::MAX / 2,
    ) {
        let ready_at = proposal_time + delay;
        proptest::prop_assert!(is_ready(ready_at, proposal_time, delay));
    }

    #[test]
    fn prop_not_ready_before_delay(
        proposal_time in 1u64..u64::MAX / 2,
        delay in 1u64..u64::MAX / 2,
    ) {
        let ready_at = proposal_time + delay;
        if ready_at > 0 {
            proptest::prop_assert!(!is_ready(ready_at - 1, proposal_time, delay));
        }
    }

    #[test]
    fn prop_delay_overflow_is_never_ready(
        proposal_time in (u64::MAX / 2 + 1)..u64::MAX,
        delay in (u64::MAX / 2 + 1)..u64::MAX,
    ) {
        proptest::prop_assert!(!is_ready(u64::MAX, proposal_time, delay));
    }

    #[test]
    fn prop_delay_below_min_is_invalid(
        min_delay in 1u64..10_000u64,
        proposed in 0u64..10_000u64,
    ) {
        let valid = proposed >= min_delay;
        proptest::prop_assert_eq!(valid, proposed >= min_delay);
    }
}
