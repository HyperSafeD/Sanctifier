//! End-to-end integration coverage for the cross-contract fuzz harness
//! target.  Exercised on every PR via `cargo test -p my-contract`.
//!
//! These tests pin down the wire format the cargo-fuzz target relies on
//! (`fuzz_targets/fuzz_cross_contract.rs`), and assert the panic-free
//! invariant on a corpus of corner-case payloads.

use my_contract::{
    cross_contract::{parse, CrossContractError, CrossContractMessage},
    handle_cross_contract_message,
};

const ADDR_A: [u8; 32] = [0x11; 32];
const ADDR_B: [u8; 32] = [0x22; 32];
const ADDR_C: [u8; 32] = [0x33; 32];

fn encode_mint(to: [u8; 32], amount: i128) -> Vec<u8> {
    let mut buf = vec![0x01];
    buf.extend_from_slice(&to);
    buf.extend_from_slice(&amount.to_le_bytes());
    buf
}

fn encode_burn(from: [u8; 32], amount: i128) -> Vec<u8> {
    let mut buf = vec![0x02];
    buf.extend_from_slice(&from);
    buf.extend_from_slice(&amount.to_le_bytes());
    buf
}

fn encode_transfer(from: [u8; 32], to: [u8; 32], amount: i128) -> Vec<u8> {
    let mut buf = vec![0x03];
    buf.extend_from_slice(&from);
    buf.extend_from_slice(&to);
    buf.extend_from_slice(&amount.to_le_bytes());
    buf
}

fn encode_approve(from: [u8; 32], spender: [u8; 32], amount: i128, exp: u32) -> Vec<u8> {
    let mut buf = vec![0x04];
    buf.extend_from_slice(&from);
    buf.extend_from_slice(&spender);
    buf.extend_from_slice(&amount.to_le_bytes());
    buf.extend_from_slice(&exp.to_le_bytes());
    buf
}

fn encode_transfer_from(spender: [u8; 32], from: [u8; 32], to: [u8; 32], amount: i128) -> Vec<u8> {
    let mut buf = vec![0x05];
    buf.extend_from_slice(&spender);
    buf.extend_from_slice(&from);
    buf.extend_from_slice(&to);
    buf.extend_from_slice(&amount.to_le_bytes());
    buf
}

#[test]
fn empty_payload_is_rejected_safely() {
    assert_eq!(
        handle_cross_contract_message(&[]),
        Err(CrossContractError::Empty)
    );
}

#[test]
fn unknown_opcode_is_rejected_safely() {
    let payload = [0xFFu8; 64];
    assert_eq!(
        handle_cross_contract_message(&payload),
        Err(CrossContractError::InvalidOpCode)
    );
}

#[test]
fn truncated_mint_payload_is_rejected_safely() {
    // opcode + 32 byte addr + only 8 bytes of the i128 amount
    let mut buf = vec![0x01];
    buf.extend_from_slice(&ADDR_A);
    buf.extend_from_slice(&[0u8; 8]);
    assert_eq!(
        handle_cross_contract_message(&buf),
        Err(CrossContractError::Truncated)
    );
}

#[test]
fn negative_amount_is_rejected_safely() {
    let buf = encode_mint(ADDR_A, -1);
    assert_eq!(
        handle_cross_contract_message(&buf),
        Err(CrossContractError::NegativeAmount)
    );
}

#[test]
fn well_formed_mint_round_trips() {
    let buf = encode_mint(ADDR_A, 1_000_000);
    let msg = parse(&buf).expect("valid mint must parse");
    assert_eq!(
        msg,
        CrossContractMessage::Mint {
            to: ADDR_A,
            amount: 1_000_000,
        }
    );
}

#[test]
fn well_formed_burn_round_trips() {
    let buf = encode_burn(ADDR_B, 42);
    assert_eq!(
        parse(&buf).expect("valid burn must parse"),
        CrossContractMessage::Burn {
            from: ADDR_B,
            amount: 42,
        }
    );
}

#[test]
fn well_formed_transfer_round_trips() {
    let buf = encode_transfer(ADDR_A, ADDR_B, 7);
    assert_eq!(
        parse(&buf).expect("valid transfer must parse"),
        CrossContractMessage::Transfer {
            from: ADDR_A,
            to: ADDR_B,
            amount: 7,
        }
    );
}

#[test]
fn well_formed_approve_round_trips() {
    let buf = encode_approve(ADDR_A, ADDR_B, 1234, 999_999);
    assert_eq!(
        parse(&buf).expect("valid approve must parse"),
        CrossContractMessage::Approve {
            from: ADDR_A,
            spender: ADDR_B,
            amount: 1234,
            expiration_ledger: 999_999,
        }
    );
}

#[test]
fn well_formed_transfer_from_round_trips() {
    let buf = encode_transfer_from(ADDR_C, ADDR_A, ADDR_B, 88);
    assert_eq!(
        parse(&buf).expect("valid transfer_from must parse"),
        CrossContractMessage::TransferFrom {
            spender: ADDR_C,
            from: ADDR_A,
            to: ADDR_B,
            amount: 88,
        }
    );
}

#[test]
fn parser_is_total_on_short_inputs() {
    // 0..=64 byte payloads filled with 0, 1, and 0xFF must never panic.
    for op in 0u16..=255 {
        for len in 0..=64usize {
            for fill in [0u8, 1, 0xFF] {
                let mut buf = vec![fill; len];
                if !buf.is_empty() {
                    buf[0] = op as u8;
                }
                // Drives the panic-free invariant — same one cargo-fuzz
                // exercises against the libFuzzer corpus.
                let _ = handle_cross_contract_message(&buf);
            }
        }
    }
}

#[test]
fn parser_handles_max_amount_boundary() {
    // i128::MAX is a valid amount and must decode cleanly.
    let buf = encode_mint(ADDR_A, i128::MAX);
    assert_eq!(
        parse(&buf).expect("i128::MAX must parse"),
        CrossContractMessage::Mint {
            to: ADDR_A,
            amount: i128::MAX,
        }
    );
}

#[test]
fn parser_rejects_min_amount() {
    // i128::MIN is negative — must be rejected without panicking on the
    // checked-sub path the contract would otherwise take.
    let buf = encode_mint(ADDR_A, i128::MIN);
    assert_eq!(
        handle_cross_contract_message(&buf),
        Err(CrossContractError::NegativeAmount)
    );
}
