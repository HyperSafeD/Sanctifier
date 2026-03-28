pub mod circuit_breaker;
pub mod guards;
pub mod state;

use guards::{require_active, GuardError};
use state::ContractState;

pub fn deposit(
    state: &mut ContractState,
    _caller: &[u8; 32],
    amount: u64,
) -> Result<(), GuardError> {
    require_active(state)?;
    state.balance = state.balance.checked_add(amount)
        .ok_or(GuardError::Unauthorized)?;
    Ok(())
}

pub fn withdraw(
    state: &mut ContractState,
    _caller: &[u8; 32],
    amount: u64,
) -> Result<(), GuardError> {
    require_active(state)?;
    if state.balance < amount {
        return Err(GuardError::Unauthorized);
    }
    state.balance -= amount;
    Ok(())
}

pub fn update_data(
    state: &mut ContractState,
    _caller: &[u8; 32],
    new_data: Vec<u8>,
) -> Result<(), GuardError> {
    require_active(state)?;
    state.data = new_data;
    Ok(())
}

pub fn get_balance(state: &ContractState) -> u64 {
    state.balance
}

pub fn get_shutdown_status(state: &ContractState) -> &state::ShutdownState {
    &state.shutdown
}
