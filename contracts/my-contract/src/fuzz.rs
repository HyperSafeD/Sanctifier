use crate::cross_contract::{handle_cross_contract_message, CrossContractMessage};
use crate::{Token, TokenClient};
use bolero::{check, generator::*};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn fresh_client(env: &Env) -> (TokenClient<'_>, Address) {
    let admin = Address::generate(env);
    let id = env.register_contract(None, Token);
    let client = TokenClient::new(env, &id);
    env.mock_all_auths();
    client.initialize(
        &admin,
        &7u32,
        &String::from_str(env, "Fuzz Token"),
        &String::from_str(env, "FUZZ"),
    );
    (client, admin)
}

#[test]
fn fuzz_mint_no_panic() {
    check!()
        .with_generator(gen::<i128>().with().bounds(0i128..=i128::MAX))
        .for_each(|amount| {
            let env = Env::default();
            let (client, _admin) = fresh_client(&env);
            let to = Address::generate(&env);
            let _ = client.try_mint(&to, amount);
        });
}

#[test]
fn fuzz_transfer_balance_conservation() {
    check!()
        .with_generator(gen::<(u32, u32)>())
        .for_each(|(mint_amount, transfer_amount)| {
            let env = Env::default();
            let (client, _admin) = fresh_client(&env);
            let alice = Address::generate(&env);
            let bob = Address::generate(&env);

            let mint_amt = *mint_amount as i128;
            let transfer_amt = *transfer_amount as i128;

            let _ = client.try_mint(&alice, &mint_amt);
            let balance_before = client.balance(&alice);

            if let Ok(Ok(())) = client.try_transfer(&alice, &bob, &transfer_amt) {
                let alice_after = client.balance(&alice);
                let bob_after = client.balance(&bob);
                assert_eq!(balance_before - transfer_amt, alice_after);
                assert_eq!(bob_after, transfer_amt);
            }
        });
}

#[test]
fn fuzz_allowance_monotone_decrease() {
    check!()
        .with_generator(gen::<(u32, u32)>())
        .for_each(|(approve_amt, draw_amt)| {
            let env = Env::default();
            let (client, _admin) = fresh_client(&env);
            let alice = Address::generate(&env);
            let bob = Address::generate(&env);
            let carol = Address::generate(&env);

            let approve = *approve_amt as i128;
            let draw = *draw_amt as i128;

            let _ = client.try_mint(&alice, &approve);
            let _ = client.try_approve(&alice, &bob, &approve, &1_000u32);

            let allowance_before = client.allowance(&alice, &bob);
            if let Ok(Ok(())) = client.try_transfer_from(&bob, &alice, &carol, &draw) {
                let allowance_after = client.allowance(&alice, &bob);
                assert!(allowance_after >= 0, "allowance went negative");
                assert_eq!(allowance_before - draw, allowance_after);
            }
        });
}

#[test]
fn fuzz_burn_balance_never_negative() {
    check!()
        .with_generator(gen::<(u32, u32)>())
        .for_each(|(mint_amt, burn_amt)| {
            let env = Env::default();
            let (client, _admin) = fresh_client(&env);
            let alice = Address::generate(&env);

            let _ = client.try_mint(&alice, &(*mint_amt as i128));

            if let Ok(Ok(())) = client.try_burn(&alice, &(*burn_amt as i128)) {
                assert!(
                    client.balance(&alice) >= 0,
                    "balance went negative after burn"
                );
            }
        });
}

/// Cross-contract harness: feeding arbitrary bytes into the parser must
/// never panic.  Either it returns a structured message or a typed error.
#[test]
fn fuzz_raw_bytes_no_panic() {
    check!()
        .with_type::<alloc::vec::Vec<u8>>()
        .for_each(|bytes| {
            let _ = handle_cross_contract_message(bytes.as_slice());
        });
}

/// Structured-message harness: round-tripping a structurally valid
/// payload through the parser must yield the same logical message.
/// This guards against silent decoder regressions.
#[test]
fn fuzz_structured_message_no_panic() {
    check!()
        .with_generator(gen::<(u8, [u8; 32], [u8; 32], [u8; 32], i128, u32)>())
        .for_each(|(op, a, b, c, amount, expiration)| {
            let mut buf = alloc::vec::Vec::with_capacity(1 + 32 * 3 + 16 + 4);
            buf.push(*op);
            buf.extend_from_slice(a);
            buf.extend_from_slice(b);
            buf.extend_from_slice(c);
            buf.extend_from_slice(&amount.to_le_bytes());
            buf.extend_from_slice(&expiration.to_le_bytes());

            // Must never panic regardless of the opcode byte.
            let parsed = handle_cross_contract_message(&buf);

            // Successful parses must round-trip: re-encoding the
            // decoded message and re-parsing yields the same value.
            if parsed.is_ok() {
                if let Ok(msg) = crate::cross_contract::parse(&buf) {
                    let reencoded = encode_message(&msg);
                    let reparsed = crate::cross_contract::parse(&reencoded)
                        .expect("re-encoded message must parse");
                    assert_eq!(msg, reparsed, "round-trip must be lossless");
                }
            }
        });
}

extern crate alloc;

fn encode_message(msg: &CrossContractMessage) -> alloc::vec::Vec<u8> {
    let mut buf = alloc::vec::Vec::new();
    match msg {
        CrossContractMessage::Mint { to, amount } => {
            buf.push(0x01);
            buf.extend_from_slice(to);
            buf.extend_from_slice(&amount.to_le_bytes());
        }
        CrossContractMessage::Burn { from, amount } => {
            buf.push(0x02);
            buf.extend_from_slice(from);
            buf.extend_from_slice(&amount.to_le_bytes());
        }
        CrossContractMessage::Transfer { from, to, amount } => {
            buf.push(0x03);
            buf.extend_from_slice(from);
            buf.extend_from_slice(to);
            buf.extend_from_slice(&amount.to_le_bytes());
        }
        CrossContractMessage::Approve {
            from,
            spender,
            amount,
            expiration_ledger,
        } => {
            buf.push(0x04);
            buf.extend_from_slice(from);
            buf.extend_from_slice(spender);
            buf.extend_from_slice(&amount.to_le_bytes());
            buf.extend_from_slice(&expiration_ledger.to_le_bytes());
        }
        CrossContractMessage::TransferFrom {
            spender,
            from,
            to,
            amount,
        } => {
            buf.push(0x05);
            buf.extend_from_slice(spender);
            buf.extend_from_slice(from);
            buf.extend_from_slice(to);
            buf.extend_from_slice(&amount.to_le_bytes());
        }
    }
    buf
}
