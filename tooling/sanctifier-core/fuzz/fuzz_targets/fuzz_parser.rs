#![no_main]

use libfuzzer_sys::fuzz_target;
use sanctifier_core::parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        // We only care that it doesn't panic.
        let _ = parser::parse_source(source);
    }
});
