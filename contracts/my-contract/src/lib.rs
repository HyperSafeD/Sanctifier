use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct CrossContractMessage {
    pub sender: [u8; 32],
    pub method_id: u8,
    pub payload: Vec<u8>,
    pub nonce: u64,
}

#[derive(Debug, PartialEq)]
pub enum ContractError {
    DeserializationFailed,
    UnknownMethod,
    InvalidPayload(String),
    OverflowDetected,
}

pub fn handle_cross_contract_message(raw: &[u8]) -> Result<String, ContractError> {
    let msg = CrossContractMessage::try_from_slice(raw)
        .map_err(|_| ContractError::DeserializationFailed)?;
    match msg.method_id {
        0 => handle_transfer(&msg.payload),
        1 => handle_query(&msg.payload),
        2 => handle_callback(&msg.payload),
        _ => Err(ContractError::UnknownMethod),
    }
}

fn handle_transfer(payload: &[u8]) -> Result<String, ContractError> {
    if payload.len() < 8 {
        return Err(ContractError::InvalidPayload("too short".into()));
    }
    let amount = u64::from_le_bytes(payload[0..8].try_into().unwrap());
    let fee = amount.checked_mul(3).ok_or(ContractError::OverflowDetected)?;
    Ok(format!("transfer: amount={amount}, fee={fee}"))
}

fn handle_query(payload: &[u8]) -> Result<String, ContractError> {
    let key = std::str::from_utf8(payload)
        .map_err(|_| ContractError::InvalidPayload("invalid utf8".into()))?;
    Ok(format!("query: key={key}"))
}

fn handle_callback(payload: &[u8]) -> Result<String, ContractError> {
    if payload.is_empty() {
        return Err(ContractError::InvalidPayload("empty callback".into()));
    }
    Ok(format!("callback: {} bytes", payload.len()))
}
