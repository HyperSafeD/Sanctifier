use bolero::{check, generator::*};
use borsh::BorshSerialize;
use my_contract::{handle_cross_contract_message, CrossContractMessage};

#[test]
fn fuzz_raw_bytes_no_panic() {
    check!()
        .with_type::<Vec<u8>>()
        .for_each(|data| {
            let _ = handle_cross_contract_message(data);
        });
}

#[test]
fn fuzz_structured_message_no_panic() {
    check!()
        .with_generator(gen::<(u8, Vec<u8>, u64)>())
        .for_each(|(method_id, payload, nonce)| {
            let msg = CrossContractMessage {
                sender: [0u8; 32],
                method_id: *method_id,
                payload: payload.clone(),
                nonce: *nonce,
            };
            if let Ok(encoded) = borsh::to_vec(&msg) {
                let _ = handle_cross_contract_message(&encoded);
            }
        });
}

#[test]
fn fuzz_truncated_messages() {
    check!()
        .with_type::<Vec<u8>>()
        .with_max_len(16)
        .for_each(|data| {
            let _ = handle_cross_contract_message(data);
        });
}

#[test]
fn fuzz_oversized_payload_no_oom() {
    check!()
        .with_generator(gen::<Vec<u8>>().with().len(0usize..=1_000_000))
        .for_each(|data| {
            let _ = handle_cross_contract_message(data);
        });
}
