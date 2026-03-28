use crate::state::{ContractState, ShutdownState};

#[derive(Debug, PartialEq)]
pub enum GuardError {
    ContractPaused,
    EmergencyShutdown,
    Unauthorized,
}

impl std::fmt::Display for GuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuardError::ContractPaused    => write!(f, "Contract is paused"),
            GuardError::EmergencyShutdown => write!(f, "Emergency shutdown is active"),
            GuardError::Unauthorized      => write!(f, "Caller is not authorized"),
        }
    }
}

pub fn require_active(state: &ContractState) -> Result<(), GuardError> {
    match state.shutdown {
        ShutdownState::Active    => Ok(()),
        ShutdownState::Paused    => Err(GuardError::ContractPaused),
        ShutdownState::Emergency => Err(GuardError::EmergencyShutdown),
    }
}

pub fn require_admin(state: &ContractState, caller: &[u8; 32]) -> Result<(), GuardError> {
    if &state.admin == caller {
        Ok(())
    } else {
        Err(GuardError::Unauthorized)
    }
}

pub fn require_admin_and_active(
    state: &ContractState,
    caller: &[u8; 32],
) -> Result<(), GuardError> {
    require_admin(state, caller)?;
    require_active(state)
}
