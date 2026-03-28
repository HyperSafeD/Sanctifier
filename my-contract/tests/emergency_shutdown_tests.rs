use my_contract::{
    circuit_breaker::{emergency_shutdown, pause, unpause},
    deposit, update_data, withdraw,
    guards::GuardError,
    state::ContractState,
};

const ADMIN: [u8; 32] = [1u8; 32];
const USER:  [u8; 32] = [2u8; 32];

fn fresh_state() -> ContractState {
    ContractState::new(ADMIN)
}

#[test]
fn test_pause_blocks_deposit() {
    let mut state = fresh_state();
    pause(&mut state, &ADMIN).unwrap();
    assert_eq!(deposit(&mut state, &USER, 100), Err(GuardError::ContractPaused));
}

#[test]
fn test_pause_blocks_withdraw() {
    let mut state = fresh_state();
    state.balance = 500;
    pause(&mut state, &ADMIN).unwrap();
    assert_eq!(withdraw(&mut state, &USER, 100), Err(GuardError::ContractPaused));
}

#[test]
fn test_pause_blocks_update_data() {
    let mut state = fresh_state();
    pause(&mut state, &ADMIN).unwrap();
    assert_eq!(update_data(&mut state, &USER, vec![1, 2, 3]), Err(GuardError::ContractPaused));
}

#[test]
fn test_reads_allowed_during_pause() {
    let mut state = fresh_state();
    state.balance = 42;
    pause(&mut state, &ADMIN).unwrap();
    assert_eq!(my_contract::get_balance(&state), 42);
}

#[test]
fn test_emergency_blocks_all_mutations() {
    let mut state = fresh_state();
    emergency_shutdown(&mut state, &ADMIN).unwrap();
    assert_eq!(deposit(&mut state, &USER, 100),         Err(GuardError::EmergencyShutdown));
    assert_eq!(withdraw(&mut state, &USER, 100),        Err(GuardError::EmergencyShutdown));
    assert_eq!(update_data(&mut state, &USER, vec![1]), Err(GuardError::EmergencyShutdown));
}

#[test]
fn test_emergency_cannot_be_unpaused() {
    let mut state = fresh_state();
    emergency_shutdown(&mut state, &ADMIN).unwrap();
    assert_eq!(unpause(&mut state, &ADMIN), Err(GuardError::EmergencyShutdown));
}

#[test]
fn test_escalation_paused_to_emergency() {
    let mut state = fresh_state();
    pause(&mut state, &ADMIN).unwrap();
    emergency_shutdown(&mut state, &ADMIN).unwrap();
    assert_eq!(deposit(&mut state, &USER, 100), Err(GuardError::EmergencyShutdown));
}

#[test]
fn test_unpause_restores_operations() {
    let mut state = fresh_state();
    pause(&mut state, &ADMIN).unwrap();
    unpause(&mut state, &ADMIN).unwrap();
    assert!(deposit(&mut state, &USER, 100).is_ok());
}

#[test]
fn test_non_admin_cannot_pause() {
    let mut state = fresh_state();
    assert_eq!(pause(&mut state, &USER), Err(GuardError::Unauthorized));
}

#[test]
fn test_non_admin_cannot_trigger_emergency() {
    let mut state = fresh_state();
    assert_eq!(emergency_shutdown(&mut state, &USER), Err(GuardError::Unauthorized));
}
