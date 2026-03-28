#![no_main]

use libfuzzer_sys::fuzz_target;
use my_contract::handle_cross_contract_message;

fuzz_target!(|data: &[u8]| {
    let _ = handle_cross_contract_message(data);
});
