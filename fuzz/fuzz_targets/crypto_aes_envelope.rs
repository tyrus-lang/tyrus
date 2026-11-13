#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| tyrus_runtime::crypto_fuzz::aes_envelope(data));
