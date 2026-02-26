use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone, PartialEq)]
pub enum ShutdownState {
    Active,
    Paused,
    Emergency
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct ContractState {
    pub admin: [u8; 32],
    pub shutdown: ShutdownState,
    pub balance: u64,
    pub data: Vec<u8>,
}

impl ContractState {
    pub fn new(admin: [u8; 32]) -> Self {
        Self {
            admin,
            shutdown: ShutdownState::Active,
            balance: 0,
            data: vec![],
        }
    }
}
