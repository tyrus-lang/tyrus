#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| tyrus_runtime::codec_fuzz::compress_and_decompress(data));
