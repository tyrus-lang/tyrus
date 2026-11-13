use super::{
    CompressResult, DecompressOptions, STREAM_CAPACITY_BYTES, compress, compression_transform,
    decompress, decompression_transform, message_for,
};
use crate::abi::TyrusArray;
use crate::executor::executor;
use crate::promise::TyrusPromise;
use crate::stream::{
    ByteTransform, StreamError, TyrusTransformStream, byte_transform_stream,
    failed_byte_transform_stream,
};
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use tyrus_string::TyrusString;

/// Layout of `CompressOptions` as the compiler lowers it: every field is
/// optional, so each is a pointer to its value, null when omitted.
#[repr(C)]
pub struct CompressOptionsFfi {
    pub level: *const i32,
}

#[repr(C)]
pub struct DecompressOptionsFfi {
    pub format: *const i32,
    pub max_output_bytes: *const u64,
    pub max_ratio: *const u32,
}

#[repr(C)]
pub struct CompressErrorFfi {
    pub code: TyrusString,
    pub message: TyrusString,
}

/// The largest variant of `Result<u8[], CompressError>` is the 32-byte error,
/// in whole `u64` words.
const RESULT_PAYLOAD_WORDS: usize = 4;

/// Layout of `Result<u8[], CompressError>` as the compiler lowers any
/// `Result`: a one-byte tag (`0` for `Ok`, `1` for `Err`) followed, at the
/// next 8-byte boundary, by the payload of the chosen variant.
#[repr(C)]
pub struct CompressResultFfi {
    pub tag: u8,
    pub payload: CompressResultPayload,
}

#[repr(C)]
pub union CompressResultPayload {
    pub ok: ManuallyDrop<TyrusArray>,
    pub err: ManuallyDrop<CompressErrorFfi>,
    words: [u64; RESULT_PAYLOAD_WORDS],
}

const RESULT_TAG_OK: u8 = 0;
const RESULT_TAG_ERR: u8 = 1;

const _: () = {
    assert!(std::mem::size_of::<CompressErrorFfi>() == 32);
    assert!(std::mem::size_of::<CompressResultFfi>() == 40);
    assert!(std::mem::offset_of!(CompressResultFfi, payload) == 8);
};

/// Copies the caller's bytes before the call returns: the program may release
/// its buffer while the work is still running on the blocking pool.
unsafe fn input_bytes(data: *const u8, length: usize) -> Vec<u8> {
    unsafe {
        if data.is_null() || length == 0 {
            return Vec::new();
        }
        std::slice::from_raw_parts(data, length).to_vec()
    }
}

unsafe fn read_level(options: *const CompressOptionsFfi) -> Option<i32> {
    unsafe {
        options
            .as_ref()
            .and_then(|options| options.level.as_ref().copied())
    }
}

unsafe fn read_decompress_options(options: *const DecompressOptionsFfi) -> DecompressOptions {
    unsafe {
        let Some(options) = options.as_ref() else {
            return DecompressOptions::default();
        };
        DecompressOptions {
            format: options.format.as_ref().copied(),
            max_output_bytes: options.max_output_bytes.as_ref().copied(),
            max_ratio: options.max_ratio.as_ref().copied(),
        }
    }
}

unsafe fn byte_array(bytes: &[u8]) -> Option<TyrusArray> {
    unsafe {
        let capacity = bytes.len().max(1);
        let data = libc::malloc(capacity).cast::<u8>();
        if data.is_null() {
            return None;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
        Some(TyrusArray {
            data,
            length: bytes.len(),
            capacity,
        })
    }
}

/// Releases what a result holds, never the result's own storage. `code` and
/// `message` are static literals, so an `Err` holds nothing to release.
unsafe fn release(result: *mut CompressResultFfi) {
    unsafe {
        if (*result).tag == RESULT_TAG_OK {
            let array = &*(*result).payload.ok;
            if !array.data.is_null() {
                libc::free(array.data.cast());
            }
        }
    }
}

unsafe extern "C" fn drop_result(value: *mut c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        release(value.cast());
        libc::free(value);
    }
}

fn error_result(code: &'static str) -> CompressResultFfi {
    CompressResultFfi {
        tag: RESULT_TAG_ERR,
        payload: CompressResultPayload {
            err: ManuallyDrop::new(CompressErrorFfi {
                code: TyrusString::literal(code),
                message: TyrusString::literal(message_for(code)),
            }),
        },
    }
}

/// Fulfils `promise` with a boxed `Result`. When the box itself cannot be
/// allocated there is no value left to deliver, so the promise is rejected
/// with the structured allocation error instead.
unsafe fn fulfill(promise: *mut TyrusPromise, outcome: CompressResult<Vec<u8>>) {
    unsafe {
        let value = match outcome {
            Ok(bytes) => match byte_array(&bytes) {
                Some(array) => CompressResultFfi {
                    tag: RESULT_TAG_OK,
                    payload: CompressResultPayload {
                        ok: ManuallyDrop::new(array),
                    },
                },
                None => error_result(super::ERR_ALLOCATION_FAILED),
            },
            Err(code) => error_result(code),
        };
        let boxed =
            libc::malloc(std::mem::size_of::<CompressResultFfi>()).cast::<CompressResultFfi>();
        if boxed.is_null() {
            let mut value = value;
            release(&mut value);
            crate::tyrus_promise_reject_pending_structured(
                promise,
                TyrusString::literal(super::ERR_ALLOCATION_FAILED),
                TyrusString::literal(message_for(super::ERR_ALLOCATION_FAILED)),
            );
            return;
        }
        std::ptr::write(boxed, value);
        crate::tyrus_promise_fulfill_owned(promise, boxed.cast(), Some(drop_result));
    }
}

/// Runs `work` on the blocking pool: compression is CPU-bound and must not
/// hold the executor threads that poll the program's async functions.
unsafe fn settle_in_background<W>(work: W) -> *mut TyrusPromise
where
    W: FnOnce() -> CompressResult<Vec<u8>> + Send + 'static,
{
    unsafe {
        let promise = crate::tyrus_promise_pending();
        crate::tyrus_promise_retain(promise);
        let address = promise as usize;
        executor().spawn(async move {
            let outcome = tokio::task::spawn_blocking(work)
                .await
                .unwrap_or(Err(super::ERR_ALLOCATION_FAILED));
            let promise = address as *mut TyrusPromise;
            fulfill(promise, outcome);
            crate::tyrus_promise_free(promise);
        });
        promise
    }
}

/// # Safety
///
/// `data` must point to `length` readable bytes (or be null with `length`
/// zero), and `options` must be null or point to a `CompressOptions` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_compress_compress(
    data: *const u8,
    length: usize,
    format: i32,
    options: *const CompressOptionsFfi,
) -> *mut TyrusPromise {
    unsafe {
        let input = input_bytes(data, length);
        let level = read_level(options);
        settle_in_background(move || compress(&input, format, level))
    }
}

/// # Safety
///
/// `data` must point to `length` readable bytes (or be null with `length`
/// zero), and `options` must be null or point to a `DecompressOptions` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_compress_decompress(
    data: *const u8,
    length: usize,
    options: *const DecompressOptionsFfi,
) -> *mut TyrusPromise {
    unsafe {
        let input = input_bytes(data, length);
        let options = read_decompress_options(options);
        settle_in_background(move || decompress(&input, options))
    }
}

/// An owned `TransformStream` reference for the program. A configuration the
/// codec refuses gives a stream whose sides fail with that error on first use,
/// since the signature has no `Result` to carry it.
fn transform_handle(
    transform: CompressResult<Box<dyn ByteTransform>>,
) -> *const TyrusTransformStream {
    let stream = match transform {
        Ok(transform) => byte_transform_stream(STREAM_CAPACITY_BYTES, transform),
        Err(code) => failed_byte_transform_stream(
            STREAM_CAPACITY_BYTES,
            StreamError {
                code,
                message: message_for(code),
            },
        ),
    };
    std::sync::Arc::into_raw(stream)
}

/// # Safety
///
/// `options` must be null or point to a `CompressOptions` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_compress_compression_stream(
    format: i32,
    options: *const CompressOptionsFfi,
) -> *const TyrusTransformStream {
    unsafe {
        let level = read_level(options);
        transform_handle(
            compression_transform(format, level)
                .map(|transform| Box::new(transform) as Box<dyn ByteTransform>),
        )
    }
}

/// # Safety
///
/// `options` must be null or point to a `DecompressOptions` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_compress_decompression_stream(
    options: *const DecompressOptionsFfi,
) -> *const TyrusTransformStream {
    unsafe {
        let options = read_decompress_options(options);
        transform_handle(
            decompression_transform(options)
                .map(|transform| Box::new(transform) as Box<dyn ByteTransform>),
        )
    }
}
