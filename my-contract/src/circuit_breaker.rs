use crate::{
    guards::{require_admin, require_active, GuardError},
    state::{ContractState, ShutdownState},
};

pub fn pause(state: &mut ContractState, caller: &[u8; 32]) -> Result<(), GuardError> {
    require_admin(state, caller)?;
    require_active(state)?;
    state.shutdown = ShutdownState::Paused;
    Ok(())
}

pub fn emergency_shutdown(
    state: &mut ContractState,
    caller: &[u8; 32],
) -> Result<(), GuardError> {
    require_admin(state, caller)?;
    state.shutdown = ShutdownState::Emergency;
    Ok(())
}

pub fn unpause(state: &mut ContractState, caller: &[u8; 32]) -> Result<(), GuardError> {
    require_admin(state, caller)?;
    match state.shutdown {
        ShutdownState::Paused    => { state.shutdown = ShutdownState::Active; Ok(()) }
        ShutdownState::Emergency => Err(GuardError::EmergencyShutdown),
        ShutdownState::Active    => Ok(()),
    }
}
