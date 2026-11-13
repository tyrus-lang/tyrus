use super::ffi::*;
use super::*;
use crate::promise::TyrusPromise;

unsafe fn settled(promise: *mut TyrusPromise) -> &'static CompressResultFfi {
    unsafe {
        crate::wait_until_settled(promise);
        assert_eq!(
            crate::tyrus_promise_state(promise),
            crate::TYRUS_PROMISE_FULFILLED,
            "compress promises fulfil with a Result, they never reject"
        );
        &*crate::promise::tyrus_promise_await(promise).cast::<CompressResultFfi>()
    }
}

unsafe fn ok_bytes(result: &CompressResultFfi) -> Vec<u8> {
    unsafe {
        assert_eq!(result.tag, 0, "expected Ok");
        let array = &*result.payload.ok;
        std::slice::from_raw_parts(array.data, array.length).to_vec()
    }
}

unsafe fn err_code(result: &CompressResultFfi) -> (&str, &str) {
    unsafe {
        assert_eq!(result.tag, 1, "expected Err");
        let error = &*result.payload.err;
        (error.code.as_str(), error.message.as_str())
    }
}

#[test]
fn compress_and_decompress_fulfil_with_ok_bytes() {
    let text = b"Hello, Tyrus!";
    unsafe {
        let level = 9;
        let options = CompressOptionsFfi { level: &level };
        let packed = tyrus_compress_compress(text.as_ptr(), text.len(), 0, &options);
        let bytes = ok_bytes(settled(packed));
        crate::tyrus_promise_free(packed);
        let restored = tyrus_compress_decompress(bytes.as_ptr(), bytes.len(), std::ptr::null());
        assert_eq!(ok_bytes(settled(restored)), text);
        crate::tyrus_promise_free(restored);
        let empty = tyrus_compress_compress(std::ptr::null(), 0, 3, std::ptr::null());
        assert!(!ok_bytes(settled(empty)).is_empty());
        crate::tyrus_promise_free(empty);
    }
}

#[test]
fn failures_fulfil_with_an_err_carrying_code_and_message() {
    unsafe {
        let level = 12;
        let options = CompressOptionsFfi { level: &level };
        let promise = tyrus_compress_compress(b"x".as_ptr(), 1, 3, &options);
        assert_eq!(
            err_code(settled(promise)),
            (
                "ERR_COMPRESS_INVALID_LEVEL",
                "the compression level is outside the range of the format"
            )
        );
        crate::tyrus_promise_free(promise);

        let promise = tyrus_compress_decompress(b"plain".as_ptr(), 5, std::ptr::null());
        assert_eq!(err_code(settled(promise)).0, "ERR_COMPRESS_UNKNOWN_FORMAT");
        crate::tyrus_promise_free(promise);
    }
}

#[test]
fn decompress_options_reach_the_codec() {
    let zeros = vec![0_u8; 2 * 1024 * 1024];
    let packed = crate::codec::compress(&zeros, crate::codec::Format::Brotli, None).unwrap();
    unsafe {
        let format = 3;
        let small: u64 = 1024;
        let options = DecompressOptionsFfi {
            format: &format,
            max_output_bytes: &small,
            max_ratio: std::ptr::null(),
        };
        let promise = tyrus_compress_decompress(packed.as_ptr(), packed.len(), &options);
        assert_eq!(err_code(settled(promise)).0, "ERR_COMPRESS_OUTPUT_LIMIT");
        crate::tyrus_promise_free(promise);

        let options = DecompressOptionsFfi {
            format: &format,
            max_output_bytes: std::ptr::null(),
            max_ratio: std::ptr::null(),
        };
        let promise = tyrus_compress_decompress(packed.as_ptr(), packed.len(), &options);
        assert_eq!(err_code(settled(promise)).0, "ERR_COMPRESS_RATIO_LIMIT");
        crate::tyrus_promise_free(promise);

        let ratio = 1_000_000;
        let options = DecompressOptionsFfi {
            format: &format,
            max_output_bytes: std::ptr::null(),
            max_ratio: &ratio,
        };
        let promise = tyrus_compress_decompress(packed.as_ptr(), packed.len(), &options);
        assert_eq!(ok_bytes(settled(promise)).len(), zeros.len());
        crate::tyrus_promise_free(promise);
    }
}

#[test]
fn a_zero_limit_is_an_invalid_option_and_never_switches_the_protection_off() {
    assert_eq!(
        decompress(
            b"",
            DecompressOptions {
                max_ratio: Some(0),
                ..DecompressOptions::default()
            }
        ),
        Err(ERR_INVALID_OPTION)
    );
    assert_eq!(
        decompress(
            b"",
            DecompressOptions {
                max_output_bytes: Some(0),
                ..DecompressOptions::default()
            }
        ),
        Err(ERR_INVALID_OPTION)
    );
    assert_eq!(compress(b"x", 7, None), Err(ERR_INVALID_OPTION));
    assert_eq!(
        decompress(
            b"x",
            DecompressOptions {
                format: Some(-1),
                ..DecompressOptions::default()
            }
        ),
        Err(ERR_INVALID_OPTION)
    );
}

#[test]
fn every_code_has_its_own_message() {
    let codes = [
        ERR_INVALID_DATA,
        ERR_CHECKSUM_MISMATCH,
        ERR_UNKNOWN_FORMAT,
        ERR_INVALID_LEVEL,
        ERR_INVALID_OPTION,
        ERR_OUTPUT_LIMIT,
        ERR_RATIO_LIMIT,
        ERR_ALLOCATION_FAILED,
    ];
    let messages: std::collections::HashSet<_> =
        codes.iter().map(|code| message_for(code)).collect();
    assert_eq!(messages.len(), codes.len());
    assert!(!messages.contains("compression failed"));
}

mod streams {
    use super::super::ffi::*;
    use crate::stream::{
        TyrusTransformStream, close_owned_writable_stream, read_owned_byte_stream,
        write_owned_byte_stream,
    };
    use std::sync::Arc;

    fn sides(
        handle: *const TyrusTransformStream,
    ) -> (
        Arc<crate::stream::TyrusReadableStream>,
        Arc<crate::stream::TyrusWritableStream>,
    ) {
        let transform = unsafe { Arc::from_raw(handle) };
        transform.sides()
    }

    async fn run(
        handle: *const TyrusTransformStream,
        input: &[u8],
    ) -> Result<Vec<u8>, &'static str> {
        let (readable, writable) = sides(handle);
        let reader = tokio::spawn(async move {
            let mut output = Vec::new();
            loop {
                match read_owned_byte_stream(&readable).await {
                    Ok(Some(chunk)) => {
                        assert!(
                            chunk.len() <= 64 * 1024,
                            "a piece never exceeds the capacity"
                        );
                        output.extend(chunk);
                    }
                    Ok(None) => return Ok(output),
                    Err(error) => return Err(error.code),
                }
            }
        });
        for chunk in input.chunks(10_000) {
            if write_owned_byte_stream(&writable, chunk.to_vec(), 64 * 1024)
                .await
                .is_err()
            {
                break;
            }
        }
        close_owned_writable_stream(&writable);
        reader.await.unwrap()
    }

    fn sample() -> Vec<u8> {
        (0..30_000_u32)
            .flat_map(|index| format!("streamed line {index}\n").into_bytes())
            .collect()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_compression_stream_feeds_a_decompression_stream() {
        let data = sample();
        for format in 0..4 {
            let packed = run(
                unsafe { tyrus_compress_compression_stream(format, std::ptr::null()) },
                &data,
            )
            .await
            .unwrap();
            let options = DecompressOptionsFfi {
                format: &format,
                max_output_bytes: std::ptr::null(),
                max_ratio: std::ptr::null(),
            };
            let restored = run(
                unsafe { tyrus_compress_decompression_stream(&options) },
                &packed,
            )
            .await
            .unwrap();
            assert_eq!(restored, data, "format {format}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_bomb_fails_the_stream_with_the_module_error() {
        let zeros = vec![0_u8; 32 * 1024 * 1024];
        let bomb = crate::codec::compress(&zeros, crate::codec::Format::Gzip, Some(9)).unwrap();
        assert_eq!(
            run(
                unsafe { tyrus_compress_decompression_stream(std::ptr::null()) },
                &bomb
            )
            .await,
            Err("ERR_COMPRESS_RATIO_LIMIT")
        );
        assert_eq!(
            run(
                unsafe { tyrus_compress_decompression_stream(std::ptr::null()) },
                b"plain text"
            )
            .await,
            Err("ERR_COMPRESS_UNKNOWN_FORMAT")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_invalid_configuration_fails_both_sides_on_first_use() {
        let level = 12;
        let options = CompressOptionsFfi { level: &level };
        let (readable, writable) = sides(unsafe { tyrus_compress_compression_stream(3, &options) });
        assert_eq!(
            read_owned_byte_stream(&readable)
                .await
                .map_err(|error| error.code),
            Err("ERR_COMPRESS_INVALID_LEVEL")
        );
        assert_eq!(
            write_owned_byte_stream(&writable, vec![1], 64)
                .await
                .map_err(|error| error.code),
            Err("ERR_COMPRESS_INVALID_LEVEL")
        );
        let zero = 0_u32;
        let options = DecompressOptionsFfi {
            format: std::ptr::null(),
            max_output_bytes: std::ptr::null(),
            max_ratio: &zero,
        };
        let (readable, _writable) = sides(unsafe { tyrus_compress_decompression_stream(&options) });
        assert_eq!(
            read_owned_byte_stream(&readable)
                .await
                .map_err(|error| error.code),
            Err("ERR_COMPRESS_INVALID_OPTION")
        );
    }
}
