//! Cross-contract message parsing surface.
//!
//! This module decodes raw byte payloads coming from another contract or an
//! off-chain caller into structured operations.  It is exposed as the fuzzing
//! harness entry point: `handle_cross_contract_message`.
//!
//! The parser is intentionally `no_std` and pure — it never panics, never
//! allocates, and only validates input.  Executing the decoded operation is
//! left to the contract-level dispatch, which already enforces auth and
//! storage invariants.
//!
//! Wire format (all integers little-endian):
//!   byte  0     : opcode
//!   bytes 1..   : opcode-specific arguments
//!
//! Opcodes:
//!   0x01 Mint        : 32B(to)            + 16B(amount: i128)
//!   0x02 Burn        : 32B(from)          + 16B(amount: i128)
//!   0x03 Transfer    : 32B(from) + 32B(to) + 16B(amount: i128)
//!   0x04 Approve     : 32B(from) + 32B(spender) + 16B(amount) + 4B(expiration: u32)
//!   0x05 TransferFrom: 32B(spender) + 32B(from) + 32B(to) + 16B(amount)

/// Result of parsing a cross-contract payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossContractError {
    /// Empty payload — no opcode byte present.
    Empty,
    /// Opcode is recognised but the body is shorter than expected.
    Truncated,
    /// Opcode byte does not correspond to any supported operation.
    InvalidOpCode,
    /// Decoded amount is negative.  Token semantics forbid negatives.
    NegativeAmount,
}

/// A structured cross-contract operation decoded from raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossContractMessage {
    Mint {
        to: [u8; 32],
        amount: i128,
    },
    Burn {
        from: [u8; 32],
        amount: i128,
    },
    Transfer {
        from: [u8; 32],
        to: [u8; 32],
        amount: i128,
    },
    Approve {
        from: [u8; 32],
        spender: [u8; 32],
        amount: i128,
        expiration_ledger: u32,
    },
    TransferFrom {
        spender: [u8; 32],
        from: [u8; 32],
        to: [u8; 32],
        amount: i128,
    },
}

const OP_MINT: u8 = 0x01;
const OP_BURN: u8 = 0x02;
const OP_TRANSFER: u8 = 0x03;
const OP_APPROVE: u8 = 0x04;
const OP_TRANSFER_FROM: u8 = 0x05;

const ADDR_LEN: usize = 32;
const I128_LEN: usize = 16;
const U32_LEN: usize = 4;

#[inline]
fn read_addr(buf: &[u8], offset: usize) -> Option<[u8; ADDR_LEN]> {
    let end = offset.checked_add(ADDR_LEN)?;
    if buf.len() < end {
        return None;
    }
    let mut out = [0u8; ADDR_LEN];
    out.copy_from_slice(&buf[offset..end]);
    Some(out)
}

#[inline]
fn read_i128(buf: &[u8], offset: usize) -> Option<i128> {
    let end = offset.checked_add(I128_LEN)?;
    if buf.len() < end {
        return None;
    }
    let mut bytes = [0u8; I128_LEN];
    bytes.copy_from_slice(&buf[offset..end]);
    Some(i128::from_le_bytes(bytes))
}

#[inline]
fn read_u32(buf: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(U32_LEN)?;
    if buf.len() < end {
        return None;
    }
    let mut bytes = [0u8; U32_LEN];
    bytes.copy_from_slice(&buf[offset..end]);
    Some(u32::from_le_bytes(bytes))
}

/// Decode `data` into a [`CrossContractMessage`].
///
/// This function is total: every input either succeeds or returns a
/// [`CrossContractError`].  It is the primary fuzz-harness target.
pub fn parse(data: &[u8]) -> Result<CrossContractMessage, CrossContractError> {
    let (op, body) = data.split_first().ok_or(CrossContractError::Empty)?;
    match *op {
        OP_MINT => {
            let to = read_addr(body, 0).ok_or(CrossContractError::Truncated)?;
            let amount = read_i128(body, ADDR_LEN).ok_or(CrossContractError::Truncated)?;
            if amount < 0 {
                return Err(CrossContractError::NegativeAmount);
            }
            Ok(CrossContractMessage::Mint { to, amount })
        }
        OP_BURN => {
            let from = read_addr(body, 0).ok_or(CrossContractError::Truncated)?;
            let amount = read_i128(body, ADDR_LEN).ok_or(CrossContractError::Truncated)?;
            if amount < 0 {
                return Err(CrossContractError::NegativeAmount);
            }
            Ok(CrossContractMessage::Burn { from, amount })
        }
        OP_TRANSFER => {
            let from = read_addr(body, 0).ok_or(CrossContractError::Truncated)?;
            let to = read_addr(body, ADDR_LEN).ok_or(CrossContractError::Truncated)?;
            let amount = read_i128(body, ADDR_LEN * 2).ok_or(CrossContractError::Truncated)?;
            if amount < 0 {
                return Err(CrossContractError::NegativeAmount);
            }
            Ok(CrossContractMessage::Transfer { from, to, amount })
        }
        OP_APPROVE => {
            let from = read_addr(body, 0).ok_or(CrossContractError::Truncated)?;
            let spender = read_addr(body, ADDR_LEN).ok_or(CrossContractError::Truncated)?;
            let amount = read_i128(body, ADDR_LEN * 2).ok_or(CrossContractError::Truncated)?;
            let expiration_ledger =
                read_u32(body, ADDR_LEN * 2 + I128_LEN).ok_or(CrossContractError::Truncated)?;
            if amount < 0 {
                return Err(CrossContractError::NegativeAmount);
            }
            Ok(CrossContractMessage::Approve {
                from,
                spender,
                amount,
                expiration_ledger,
            })
        }
        OP_TRANSFER_FROM => {
            let spender = read_addr(body, 0).ok_or(CrossContractError::Truncated)?;
            let from = read_addr(body, ADDR_LEN).ok_or(CrossContractError::Truncated)?;
            let to = read_addr(body, ADDR_LEN * 2).ok_or(CrossContractError::Truncated)?;
            let amount = read_i128(body, ADDR_LEN * 3).ok_or(CrossContractError::Truncated)?;
            if amount < 0 {
                return Err(CrossContractError::NegativeAmount);
            }
            Ok(CrossContractMessage::TransferFrom {
                spender,
                from,
                to,
                amount,
            })
        }
        _ => Err(CrossContractError::InvalidOpCode),
    }
}

/// Top-level fuzz-harness entry: parse and validate raw bytes.
///
/// Returns `Ok(())` when the payload decodes to a structurally valid
/// message; otherwise the parse error.  This function is the panic-free
/// invariant exercised by `fuzz_cross_contract`.
pub fn handle_cross_contract_message(data: &[u8]) -> Result<(), CrossContractError> {
    parse(data).map(|_| ())
}
