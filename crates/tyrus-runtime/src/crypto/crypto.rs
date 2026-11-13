//! Native crypto runtime functions exposed via C ABI for `tyrus:crypto`.
//!
//! # FFI safety contract
//!
//! Generated callers must pass live, correctly aligned pointers for the duration of each call.
//! C strings must be NUL-terminated, byte pointers may be null only when their length is zero,
//! and output pointers must address writable `TyrusArray` storage. This module validates nullness
//! and resource limits before dereferencing, but no native callee can prove that a non-null raw
//! pointer belongs to a live allocation.

use super::*;
use futures_util::StreamExt;
use sha2::Digest;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tyrus_string::TyrusString;
use zeroize::{Zeroize, Zeroizing};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    #[cfg_attr(not(test), allow(dead_code))]
    fn free(pointer: *mut c_void);
}

unsafe fn crypto_malloc(size: usize) -> *mut c_void {
    unsafe {
        #[cfg(feature = "benchmarking")]
        crate::crypto_bench::record_ffi_allocation(size);
        malloc(size)
    }
}

const MAX_CRYPTO_INPUT: usize = 64 * 1024 * 1024;
const MAX_RANDOM_BYTES: usize = 16 * 1024 * 1024;
const MAX_PASSWORD_BYTES: usize = 1024;
const MAX_PEPPER_BYTES: usize = 1024;
const MAX_PHC_BYTES: usize = 4096;
const MAX_JWT_BYTES: usize = 1024 * 1024;
const MAX_JWT_SECRET_BYTES: usize = 64 * 1024;
const ARGON2_MEMORY_COST: u32 = 65_536;
const ARGON2_TIME_COST: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;
const ARGON2_SALT_LENGTH: usize = 16;
const ARGON2_OUTPUT_LENGTH: usize = 32;
const AES_KEY_LENGTH: usize = 32;
const AES_NONCE_LENGTH: usize = 12;
const AES_TAG_LENGTH: usize = 16;
const AES_ENVELOPE_HEADER: &[u8; 7] = b"TYRUS\x01\x01";

#[repr(C)]
pub struct CryptoErrorFfi {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
pub struct CryptoResultFfi<T> {
    pub tag: u8,
    pub payload: CryptoResultPayload<T>,
}

#[repr(C)]
pub union CryptoResultPayload<T> {
    pub ok: std::mem::ManuallyDrop<T>,
    pub err: std::mem::ManuallyDrop<CryptoErrorFfi>,
    words: [u64; 4],
}

const _: () = {
    assert!(std::mem::size_of::<CryptoErrorFfi>() == 32);
    assert!(std::mem::size_of::<CryptoResultFfi<TyrusString>>() == 40);
    assert!(std::mem::size_of::<CryptoResultFfi<TyrusArray>>() == 40);
    assert!(std::mem::size_of::<CryptoResultFfi<*mut ()>>() == 40);
    assert!(std::mem::size_of::<CryptoResultFfi<i64>>() == 40);
    assert!(std::mem::size_of::<CryptoResultFfi<u8>>() == 40);
    assert!(std::mem::offset_of!(CryptoResultFfi<TyrusString>, payload) == 8);
    assert!(std::mem::offset_of!(CryptoResultFfi<*mut ()>, payload) == 8);
    assert!(std::mem::offset_of!(CryptoResultFfi<i64>, payload) == 8);
    assert!(std::mem::offset_of!(CryptoResultFfi<u8>, payload) == 8);
};

thread_local! {
    static LAST_CRYPTO_ERROR: Cell<Option<&'static CStr>> = const { Cell::new(None) };
}

fn set_crypto_error(error: &'static CStr) {
    LAST_CRYPTO_ERROR.with(|slot| slot.set(Some(error)));
}

fn crypto_error_message(code: &CStr) -> &'static str {
    match code.to_bytes() {
        b"ERR_CRYPTO_INVALID_INPUT" => "input is invalid or exceeds the supported size",
        b"ERR_CRYPTO_DIGEST_ALGORITHM" => "digest algorithm is unsupported",
        b"ERR_CRYPTO_INVALID_STREAM" => "digest stream is invalid",
        b"ERR_CRYPTO_INVALID_OUTPUT" => "digest output is invalid",
        b"ERR_CRYPTO_INVALID_KEY_SIZE" => "AES-256-GCM requires a 32-byte key",
        b"ERR_CRYPTO_INVALID_ENVELOPE" => "ciphertext envelope is malformed",
        b"ERR_CRYPTO_DECRYPTION_FAILED" => "ciphertext authentication failed",
        b"ERR_CRYPTO_INVALID_UTF8" => "decrypted plaintext is not valid UTF-8",
        b"ERR_CRYPTO_ENCRYPTION_FAILED" => "AES-GCM encryption failed",
        b"ERR_CRYPTO_INVALID_KEY" => "HMAC key is invalid",
        b"ERR_CRYPTO_INVALID_RANGE" => "minimum must not be greater than maximum",
        b"ERR_CRYPTO_RANDOM_LIMIT" => "requested random byte length exceeds the 16 MiB limit",
        b"ERR_CRYPTO_RANDOM_FAILED" => "secure random number generation failed",
        b"ERR_CRYPTO_ALLOCATION_FAILED" => "unable to allocate the requested random bytes",
        b"ERR_CRYPTO_JWT_INVALID_CLAIMS" => "JWT claims do not match the declared type",
        b"ERR_CRYPTO_JWT_INVALID_KEY" => "JWT signing or verification key is invalid",
        b"ERR_CRYPTO_JWT_INVALID_PAYLOAD" => "JWT claims payload is invalid",
        b"ERR_CRYPTO_JWT_MALFORMED" => "JWT is malformed or has an invalid signature",
        b"ERR_CRYPTO_JWT_EXPIRED" => "JWT has expired or is not yet valid",
        b"ERR_CRYPTO_JWT_INPUT_TOO_LARGE" => "JWT input exceeds the supported size",
        b"ERR_CRYPTO_JWT_INVALID_ALGORITHM" => {
            "JWT algorithm is not allowed or does not match its key"
        }
        b"ERR_CRYPTO_JWT_INVALID_OPTION" => "JWT options are invalid",
        b"ERR_CRYPTO_JWT_INVALID_SIGNATURE" => "JWT signature verification failed",
        b"ERR_CRYPTO_JWT_MISSING_KID" => "JWT header is missing a key identifier",
        b"ERR_CRYPTO_JWT_SIGN_FAILED" => "JWT signing failed",
        b"ERR_CRYPTO_JWT_UNSUPPORTED_CRITICAL_HEADER" => "JWT uses an unsupported critical header",
        b"ERR_CRYPTO_JWT_WEAK_KEY" => "JWT key does not meet the minimum strength requirement",
        b"ERR_CRYPTO_CLOCK_FAILED" => "system clock is before the Unix epoch",
        b"ERR_CRYPTO_JWKS_INVALID" => "JWKS document is invalid",
        b"ERR_CRYPTO_JWKS_DUPLICATE_KID" => "JWKS contains duplicate key identifiers",
        b"ERR_CRYPTO_JWKS_KEY_NOT_FOUND" => "JWKS contains no compatible key for the token",
        b"ERR_CRYPTO_JWKS_INVALID_URL" => "JWKS URL is invalid or violates the HTTPS policy",
        b"ERR_CRYPTO_JWKS_FORBIDDEN_ADDRESS" => "JWKS host resolves to a forbidden address",
        b"ERR_CRYPTO_JWKS_TIMEOUT" => "JWKS request timed out",
        b"ERR_CRYPTO_JWKS_DNS_FAILED" => "JWKS host could not be resolved",
        b"ERR_CRYPTO_JWKS_FETCH_FAILED" => "JWKS request failed",
        b"ERR_CRYPTO_JWKS_HTTP_STATUS" => "JWKS server returned an unsuccessful HTTP status",
        b"ERR_CRYPTO_JWKS_TOO_LARGE" => "JWKS response exceeds the 256 KiB limit",
        b"ERR_CRYPTO_JWKS_BUSY" => "all JWKS single-flight cache slots are busy",
        b"ERR_CRYPTO_ARGON2_INVALID_HASH" => {
            "Argon2 PHC hash is invalid or exceeds the supported size"
        }
        b"ERR_CRYPTO_ARGON2_INVALID_PASSWORD" => "Argon2 password exceeds the supported size",
        b"ERR_CRYPTO_ARGON2_INVALID_PEPPER" => {
            "Argon2 pepper is invalid or exceeds the supported size"
        }
        b"ERR_CRYPTO_ARGON2_PARAMETERS" => {
            "Argon2 parameters are invalid or exceed supported limits"
        }
        b"ERR_CRYPTO_ARGON2_PEPPER_REQUIRED" => "this Argon2 hash requires its pepper",
        b"ERR_CRYPTO_ARGON2_BUSY" => "all Argon2 worker slots are busy",
        b"ERR_CRYPTO_ARGON2_WORKER_FAILED" => "Argon2 worker task failed",
        b"ERR_CRYPTO_ARGON2_FAILED" => "Argon2 operation failed",
        _ => "cryptographic operation failed",
    }
}

macro_rules! crypto_error {
    ($error:expr_2021, ()) => {{
        set_crypto_error($error);
        return;
    }};
    ($error:expr_2021, $fallback:expr_2021) => {{
        set_crypto_error($error);
        return $fallback;
    }};
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_crypto_clear_error() {
    LAST_CRYPTO_ERROR.with(|slot| slot.set(None));
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_crypto_take_error() -> TyrusString {
    LAST_CRYPTO_ERROR.with(|slot| {
        slot.take().map_or(TyrusString::null(), |error| {
            TyrusString::literal(error.to_str().expect("a static error literal is UTF-8"))
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_crypto_set_invalid_jwt_claims_error() {
    set_crypto_error(c"ERR_CRYPTO_JWT_INVALID_CLAIMS");
}

unsafe fn write_crypto_result<T>(out: *mut CryptoResultFfi<T>, value: T) {
    if out.is_null() {
        unsafe { crate::tyrus_panic(c"tyrus runtime error: null crypto Result output".as_ptr()) };
    }
    let error = LAST_CRYPTO_ERROR.with(std::cell::Cell::take);
    let result = if let Some(code) = error {
        let message = crypto_error_message(code);
        CryptoResultFfi {
            tag: 1,
            payload: CryptoResultPayload {
                err: std::mem::ManuallyDrop::new(CryptoErrorFfi {
                    code: TyrusString::literal(code.to_str().expect("static crypto error code")),
                    message: TyrusString::literal(message),
                }),
            },
        }
    } else {
        CryptoResultFfi {
            tag: 0,
            payload: CryptoResultPayload {
                ok: std::mem::ManuallyDrop::new(value),
            },
        }
    };
    unsafe { out.write(result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_take_error_result(out: *mut u8) {
    if out.is_null() {
        unsafe { crate::tyrus_panic(c"tyrus runtime error: null crypto Result output".as_ptr()) };
    }
    let Some(code) = LAST_CRYPTO_ERROR.with(std::cell::Cell::take) else {
        unsafe { crate::tyrus_panic(c"tyrus runtime error: missing crypto error".as_ptr()) };
    };
    let error = CryptoErrorFfi {
        code: TyrusString::literal(code.to_str().expect("static crypto error code is UTF-8")),
        message: TyrusString::literal(crypto_error_message(code)),
    };
    // SAFETY: the codegen caller provides a writable Result slot whose tag is at byte 0 and whose aligned payload starts at byte 8, as asserted above for the shared ABI.
    unsafe {
        out.write(1);
        out.add(8)
            .cast::<std::mem::ManuallyDrop<CryptoErrorFfi>>()
            .write(std::mem::ManuallyDrop::new(error));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_rand_ulid_result(out: *mut CryptoResultFfi<TyrusString>) {
    tyrus_crypto_clear_error();
    let value = tyrus_crypto_rand_ulid();
    unsafe { write_crypto_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_rand_random_bytes_result(
    length: u32,
    out: *mut CryptoResultFfi<TyrusArray>,
) {
    tyrus_crypto_clear_error();
    let mut value = TyrusArray {
        data: ptr::null_mut(),
        length: 0,
        capacity: 0,
    };
    unsafe { tyrus_crypto_rand_random_bytes(length, &mut value) };
    unsafe { write_crypto_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_rand_random_int_result(
    min: i64,
    max: i64,
    out: *mut CryptoResultFfi<i64>,
) {
    tyrus_crypto_clear_error();
    let value = tyrus_crypto_rand_random_int(min, max);
    unsafe { write_crypto_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_cipher_aes_encrypt_result(
    data_ptr: *const u8,
    data_len: usize,
    key_ptr: *const u8,
    key_len: usize,
    aad_ptr: *const u8,
    aad_len: usize,
    out: *mut CryptoResultFfi<TyrusArray>,
) {
    tyrus_crypto_clear_error();
    let mut value = TyrusArray {
        data: ptr::null_mut(),
        length: 0,
        capacity: 0,
    };
    unsafe {
        tyrus_crypto_cipher_aes_encrypt(
            data_ptr, data_len, key_ptr, key_len, aad_ptr, aad_len, &mut value,
        )
    };
    unsafe { write_crypto_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_cipher_aes_decrypt_str_result(
    envelope_ptr: *const u8,
    envelope_len: usize,
    key_ptr: *const u8,
    key_len: usize,
    aad_ptr: *const u8,
    aad_len: usize,
    out: *mut CryptoResultFfi<TyrusString>,
) {
    tyrus_crypto_clear_error();
    let value = unsafe {
        tyrus_crypto_cipher_aes_decrypt_str(
            envelope_ptr,
            envelope_len,
            key_ptr,
            key_len,
            aad_ptr,
            aad_len,
        )
    };
    unsafe { write_crypto_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_cipher_aes_decrypt_bytes_result(
    envelope_ptr: *const u8,
    envelope_len: usize,
    key_ptr: *const u8,
    key_len: usize,
    aad_ptr: *const u8,
    aad_len: usize,
    out: *mut CryptoResultFfi<TyrusArray>,
) {
    tyrus_crypto_clear_error();
    let mut value = TyrusArray {
        data: ptr::null_mut(),
        length: 0,
        capacity: 0,
    };
    unsafe {
        tyrus_crypto_cipher_aes_decrypt_bytes(
            envelope_ptr,
            envelope_len,
            key_ptr,
            key_len,
            aad_ptr,
            aad_len,
            &mut value,
        )
    };
    unsafe { write_crypto_result(out, value) };
}

unsafe fn cstr_to_string(ptr: TyrusString) -> Result<String, ()> {
    if ptr.is_null() {
        return Err(());
    }
    let value = unsafe { ptr.as_str() };
    if value.len() > MAX_CRYPTO_INPUT {
        return Err(());
    }
    Ok(value.to_owned())
}

fn argon2_slots() -> &'static Arc<tokio::sync::Semaphore> {
    static SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    SLOTS.get_or_init(|| Arc::new(tokio::sync::Semaphore::new(4)))
}

pub(crate) struct CheckedBytes {
    data: *const u8,
    length: usize,
}

impl std::ops::Deref for CheckedBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        if self.length == 0 {
            &[]
        } else {
            // SAFETY: `CheckedBytes` can only be created by the unsafe FFI adapter after null and
            // length validation; the generated caller guarantees the allocation remains live.
            unsafe { std::slice::from_raw_parts(self.data, self.length) }
        }
    }
}

impl AsRef<[u8]> for CheckedBytes {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

pub(crate) unsafe fn bytes_from_raw(data: *const u8, length: usize) -> Result<CheckedBytes, ()> {
    if length > MAX_CRYPTO_INPUT || (length > 0 && data.is_null()) {
        return Err(());
    }
    Ok(CheckedBytes { data, length })
}

fn fill_random_using(
    destination: &mut [u8],
    mut provider: impl FnMut(&mut [u8]) -> Result<(), ()>,
) -> Result<(), ()> {
    if provider(destination).is_err() {
        destination.zeroize();
        Err(())
    } else {
        Ok(())
    }
}

fn fill_system_random(destination: &mut [u8]) -> Result<(), ()> {
    fill_random_using(destination, |bytes| getrandom::fill(bytes).map_err(|_| ()))
}

unsafe fn write_array(output: *mut TyrusArray, data: &[u8]) {
    unsafe {
        let Some(output) = output.as_mut() else {
            crypto_error!(c"ERR_CRYPTO_INVALID_OUTPUT", ());
        };
        if data.is_empty() {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        }
        let allocation = crypto_malloc(data.len()).cast::<u8>();
        if allocation.is_null() {
            crypto_error!(c"ERR_CRYPTO_ALLOCATION_FAILED", ());
        }
        ptr::copy_nonoverlapping(data.as_ptr(), allocation, data.len());
        *output = TyrusArray {
            data: allocation,
            length: data.len(),
            capacity: data.len(),
        };
    }
}

fn crypto_error_code(code: &str) -> &'static CStr {
    match code {
        "ERR_CRYPTO_ARGON2_INVALID_HASH" => c"ERR_CRYPTO_ARGON2_INVALID_HASH",
        "ERR_CRYPTO_ARGON2_INVALID_PASSWORD" => c"ERR_CRYPTO_ARGON2_INVALID_PASSWORD",
        "ERR_CRYPTO_ARGON2_INVALID_PEPPER" => c"ERR_CRYPTO_ARGON2_INVALID_PEPPER",
        "ERR_CRYPTO_ARGON2_PARAMETERS" => c"ERR_CRYPTO_ARGON2_PARAMETERS",
        "ERR_CRYPTO_ARGON2_PEPPER_REQUIRED" => c"ERR_CRYPTO_ARGON2_PEPPER_REQUIRED",
        "ERR_CRYPTO_ARGON2_BUSY" => c"ERR_CRYPTO_ARGON2_BUSY",
        "ERR_CRYPTO_ARGON2_WORKER_FAILED" => c"ERR_CRYPTO_ARGON2_WORKER_FAILED",
        "ERR_CRYPTO_ARGON2_FAILED" => c"ERR_CRYPTO_ARGON2_FAILED",
        "ERR_CRYPTO_RANDOM_FAILED" => c"ERR_CRYPTO_RANDOM_FAILED",
        "ERR_CRYPTO_ALLOCATION_FAILED" => c"ERR_CRYPTO_ALLOCATION_FAILED",
        "ERR_CRYPTO_JWKS_INVALID" => c"ERR_CRYPTO_JWKS_INVALID",
        "ERR_CRYPTO_JWKS_DUPLICATE_KID" => c"ERR_CRYPTO_JWKS_DUPLICATE_KID",
        "ERR_CRYPTO_JWKS_KEY_NOT_FOUND" => c"ERR_CRYPTO_JWKS_KEY_NOT_FOUND",
        "ERR_CRYPTO_JWKS_INVALID_URL" => c"ERR_CRYPTO_JWKS_INVALID_URL",
        "ERR_CRYPTO_JWKS_FORBIDDEN_ADDRESS" => c"ERR_CRYPTO_JWKS_FORBIDDEN_ADDRESS",
        "ERR_CRYPTO_JWKS_TIMEOUT" => c"ERR_CRYPTO_JWKS_TIMEOUT",
        "ERR_CRYPTO_JWKS_DNS_FAILED" => c"ERR_CRYPTO_JWKS_DNS_FAILED",
        "ERR_CRYPTO_JWKS_FETCH_FAILED" => c"ERR_CRYPTO_JWKS_FETCH_FAILED",
        "ERR_CRYPTO_JWKS_HTTP_STATUS" => c"ERR_CRYPTO_JWKS_HTTP_STATUS",
        "ERR_CRYPTO_JWKS_TOO_LARGE" => c"ERR_CRYPTO_JWKS_TOO_LARGE",
        "ERR_CRYPTO_JWKS_BUSY" => c"ERR_CRYPTO_JWKS_BUSY",
        _ => c"ERR_CRYPTO_ARGON2_FAILED",
    }
}

fn crypto_error_from_code(code: &str) -> CryptoErrorFfi {
    let code = crypto_error_code(code);
    CryptoErrorFfi {
        code: TyrusString::literal(code.to_str().expect("static crypto error code")),
        message: TyrusString::literal(crypto_error_message(code)),
    }
}

unsafe extern "C" fn drop_crypto_string_result(value: *mut std::ffi::c_void) {
    unsafe {
        if value.is_null() {
            return;
        }
        let result = value.cast::<CryptoResultFfi<TyrusString>>();
        if (*result).tag == 0 {
            let string = std::mem::replace(&mut *(*result).payload.ok, TyrusString::null());
            string.release();
        }
        libc::free(value);
    }
}

unsafe extern "C" fn drop_argon2_verify_result(value: *mut std::ffi::c_void) {
    if !value.is_null() {
        unsafe { libc::free(value) };
    }
}

unsafe fn fulfill_crypto_string_result(
    promise: *mut promise::TyrusPromise,
    outcome: Result<String, String>,
) {
    unsafe {
        let outcome = outcome.and_then(|value| {
            let string = crate::strings::alloc_string(&value);
            if string.is_null() {
                Err("ERR_CRYPTO_ALLOCATION_FAILED".to_string())
            } else {
                Ok(string)
            }
        });
        let result = match outcome {
            Ok(value) => CryptoResultFfi {
                tag: 0,
                payload: CryptoResultPayload {
                    ok: std::mem::ManuallyDrop::new(value),
                },
            },
            Err(code) => CryptoResultFfi {
                tag: 1,
                payload: CryptoResultPayload {
                    err: std::mem::ManuallyDrop::new(crypto_error_from_code(&code)),
                },
            },
        };
        let slot =
            crypto_malloc(std::mem::size_of_val(&result)).cast::<CryptoResultFfi<TyrusString>>();
        if slot.is_null() {
            crate::tyrus_panic(
                c"tyrus runtime error: unable to allocate crypto Result payload".as_ptr(),
            );
        }
        slot.write(result);
        crate::tyrus_promise_fulfill_owned(promise, slot.cast(), Some(drop_crypto_string_result));
    }
}

unsafe fn fulfill_argon2_verify_result(
    promise: *mut promise::TyrusPromise,
    outcome: Result<bool, &'static str>,
) {
    unsafe {
        let result = match outcome {
            Ok(value) => CryptoResultFfi {
                tag: 0,
                payload: CryptoResultPayload {
                    ok: std::mem::ManuallyDrop::new(u8::from(value)),
                },
            },
            Err(code) => CryptoResultFfi {
                tag: 1,
                payload: CryptoResultPayload {
                    err: std::mem::ManuallyDrop::new(crypto_error_from_code(code)),
                },
            },
        };
        let slot = crypto_malloc(std::mem::size_of_val(&result)).cast::<CryptoResultFfi<u8>>();
        if slot.is_null() {
            crate::tyrus_panic(
                c"tyrus runtime error: unable to allocate Argon2 Result payload".as_ptr(),
            );
        }
        slot.write(result);
        crate::tyrus_promise_fulfill_owned(promise, slot.cast(), Some(drop_argon2_verify_result));
    }
}

unsafe fn argon2_error_promise(
    code: &'static str,
    hash_result: bool,
) -> *mut promise::TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        if hash_result {
            fulfill_crypto_string_result(promise, Err(code.to_string()));
        } else {
            fulfill_argon2_verify_result(promise, Err(code));
        }
        promise
    }
}

unsafe fn crypto_string_error_promise(code: &'static str) -> *mut promise::TyrusPromise {
    unsafe {
        let promise = crate::tyrus_promise_pending();
        fulfill_crypto_string_result(promise, Err(code.to_string()));
        promise
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_timing_safe_equal(
    a_ptr: *const u8,
    a_len: usize,
    b_ptr: *const u8,
    b_len: usize,
) -> bool {
    unsafe {
        if a_len != b_len {
            return false;
        }
        let (Ok(a), Ok(b)) = (bytes_from_raw(a_ptr, a_len), bytes_from_raw(b_ptr, b_len)) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_INPUT", false);
        };
        use subtle::ConstantTimeEq;
        a.ct_eq(&b).into()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_hmac_sha256(
    data_ptr: *const u8,
    data_len: usize,
    key_ptr: *const u8,
    key_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let (Ok(data), Ok(key)) = (
            bytes_from_raw(data_ptr, data_len),
            bytes_from_raw(key_ptr, key_len),
        ) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_INPUT", ());
        };
        let key = Zeroizing::new(key.to_vec());
        use hmac::{Hmac, KeyInit, Mac};
        type H = Hmac<sha2::Sha256>;
        let Ok(mut mac) = H::new_from_slice(&key) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_KEY", ());
        };
        mac.update(&data);
        write_array(output, &mac.finalize().into_bytes());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_hmac_sha384(
    data_ptr: *const u8,
    data_len: usize,
    key_ptr: *const u8,
    key_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let (Ok(data), Ok(key)) = (
            bytes_from_raw(data_ptr, data_len),
            bytes_from_raw(key_ptr, key_len),
        ) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_INPUT", ());
        };
        let key = Zeroizing::new(key.to_vec());
        use hmac::{Hmac, KeyInit, Mac};
        type H = Hmac<sha2::Sha384>;
        let Ok(mut mac) = H::new_from_slice(&key) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_KEY", ());
        };
        mac.update(&data);
        write_array(output, &mac.finalize().into_bytes());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_hmac_sha512(
    data_ptr: *const u8,
    data_len: usize,
    key_ptr: *const u8,
    key_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let (Ok(data), Ok(key)) = (
            bytes_from_raw(data_ptr, data_len),
            bytes_from_raw(key_ptr, key_len),
        ) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_INPUT", ());
        };
        let key = Zeroizing::new(key.to_vec());
        use hmac::{Hmac, KeyInit, Mac};
        type H = Hmac<sha2::Sha512>;
        let Ok(mut mac) = H::new_from_slice(&key) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_KEY", ());
        };
        mac.update(&data);
        write_array(output, &mac.finalize().into_bytes());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_sha256(
    data_ptr: *const u8,
    data_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        use sha2::{Digest, Sha256};
        let Ok(data) = bytes_from_raw(data_ptr, data_len) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_INPUT", ());
        };
        let mut h = Sha256::new();
        h.update(data);
        write_array(output, &h.finalize());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_sha384(
    data_ptr: *const u8,
    data_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        use sha2::{Digest, Sha384};
        let Ok(data) = bytes_from_raw(data_ptr, data_len) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_INPUT", ());
        };
        let mut h = Sha384::new();
        h.update(data);
        write_array(output, &h.finalize());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_sha512(
    data_ptr: *const u8,
    data_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        use sha2::{Digest, Sha512};
        let Ok(data) = bytes_from_raw(data_ptr, data_len) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_INPUT", ());
        };
        let mut h = Sha512::new();
        h.update(data);
        write_array(output, &h.finalize());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_md5(
    data_ptr: *const u8,
    data_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        use md5::{Digest, Md5};
        let Ok(data) = bytes_from_raw(data_ptr, data_len) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_INPUT", ());
        };
        let mut h = Md5::new();
        h.update(data);
        write_array(output, &h.finalize());
    }
}

macro_rules! hmac_result_wrapper {
    ($wrapper:ident, $operation:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $wrapper(
            data_ptr: *const u8,
            data_len: usize,
            key_ptr: *const u8,
            key_len: usize,
            out: *mut CryptoResultFfi<TyrusArray>,
        ) {
            tyrus_crypto_clear_error();
            let mut value = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            unsafe { $operation(data_ptr, data_len, key_ptr, key_len, &mut value) };
            unsafe { write_crypto_result(out, value) };
        }
    };
}

macro_rules! digest_result_wrapper {
    ($wrapper:ident, $operation:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $wrapper(
            data_ptr: *const u8,
            data_len: usize,
            out: *mut CryptoResultFfi<TyrusArray>,
        ) {
            tyrus_crypto_clear_error();
            let mut value = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            unsafe { $operation(data_ptr, data_len, &mut value) };
            unsafe { write_crypto_result(out, value) };
        }
    };
}

hmac_result_wrapper!(tyrus_crypto_hmac_sha256_result, tyrus_crypto_hmac_sha256);
hmac_result_wrapper!(tyrus_crypto_hmac_sha384_result, tyrus_crypto_hmac_sha384);
hmac_result_wrapper!(tyrus_crypto_hmac_sha512_result, tyrus_crypto_hmac_sha512);
digest_result_wrapper!(
    tyrus_crypto_digest_sha256_result,
    tyrus_crypto_digest_sha256
);
digest_result_wrapper!(
    tyrus_crypto_digest_sha384_result,
    tyrus_crypto_digest_sha384
);
digest_result_wrapper!(
    tyrus_crypto_digest_sha512_result,
    tyrus_crypto_digest_sha512
);
digest_result_wrapper!(tyrus_crypto_digest_md5_result, tyrus_crypto_digest_md5);

fn validate_random_bytes_length(length: usize) -> Result<(), &'static str> {
    if length > MAX_RANDOM_BYTES {
        Err("ERR_CRYPTO_RANDOM_LIMIT")
    } else {
        Ok(())
    }
}

fn validate_random_int_range(min: i64, max: i64) -> Result<(), &'static str> {
    if min > max {
        Err("ERR_CRYPTO_INVALID_RANGE")
    } else {
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_crypto_rand_uuidv7() -> TyrusString {
    crate::strings::alloc_string(&uuid::Uuid::now_v7().to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_crypto_rand_uuidv4() -> TyrusString {
    crate::strings::alloc_string(&uuid::Uuid::new_v4().to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_crypto_rand_ulid() -> TyrusString {
    static GENERATOR: OnceLock<Mutex<ulid::Generator>> = OnceLock::new();
    let Ok(mut generator) = GENERATOR
        .get_or_init(|| Mutex::new(ulid::Generator::new()))
        .lock()
    else {
        crypto_error!(c"ERR_CRYPTO_RANDOM_FAILED", TyrusString::null());
    };
    match generator.generate() {
        Ok(value) => crate::strings::alloc_string(&value.to_string()),
        Err(_) => crypto_error!(c"ERR_CRYPTO_RANDOM_FAILED", TyrusString::null()),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_rand_random_bytes(length: u32, output: *mut TyrusArray) {
    unsafe {
        let length = length as usize;
        if validate_random_bytes_length(length).is_err() {
            crypto_error!(c"ERR_CRYPTO_RANDOM_LIMIT", ());
        }
        let mut buf = Zeroizing::new(vec![0u8; length]);
        if fill_system_random(&mut buf).is_err() {
            crypto_error!(c"ERR_CRYPTO_RANDOM_FAILED", ());
        }
        write_array(output, &buf);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_crypto_rand_random_int(min: i64, max: i64) -> i64 {
    if validate_random_int_range(min, max).is_err() {
        crypto_error!(c"ERR_CRYPTO_INVALID_RANGE", 0);
    }
    rand::random_range(min..=max)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_crypto_rand_random_float() -> f64 {
    rand::random_range(0.0..1.0)
}

#[cfg(test)]
fn encrypt_aes_gcm_raw(
    plaintext: &[u8],
    key: &[u8],
    nonce: &[u8; AES_NONCE_LENGTH],
    aad: &[u8],
) -> Result<Vec<u8>, &'static CStr> {
    use aes_gcm::{
        Aes256Gcm, KeyInit,
        aead::{Aead, Nonce, Payload},
    };
    if key.len() != AES_KEY_LENGTH {
        return Err(c"ERR_CRYPTO_INVALID_KEY_SIZE");
    }
    let key = Zeroizing::new(key.to_vec());
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| c"ERR_CRYPTO_INVALID_KEY_SIZE")?;
    cipher
        .encrypt(
            &Nonce::<Aes256Gcm>::from(*nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| c"ERR_CRYPTO_ENCRYPTION_FAILED")
}

fn encrypt_aes_gcm_in_place(
    plaintext_and_ciphertext: &mut [u8],
    key: &[u8],
    nonce: &[u8; AES_NONCE_LENGTH],
    aad: &[u8],
) -> Result<[u8; AES_TAG_LENGTH], &'static CStr> {
    use aes_gcm::{
        Aes256Gcm, KeyInit,
        aead::{AeadInOut, Nonce},
    };
    if key.len() != AES_KEY_LENGTH {
        return Err(c"ERR_CRYPTO_INVALID_KEY_SIZE");
    }
    let key = Zeroizing::new(key.to_vec());
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| c"ERR_CRYPTO_INVALID_KEY_SIZE")?;
    let tag = cipher
        .encrypt_inout_detached(
            &Nonce::<Aes256Gcm>::from(*nonce),
            aad,
            plaintext_and_ciphertext.into(),
        )
        .map_err(|_| c"ERR_CRYPTO_ENCRYPTION_FAILED")?;
    let mut tag_bytes = [0u8; AES_TAG_LENGTH];
    tag_bytes.copy_from_slice(&tag);
    Ok(tag_bytes)
}

fn decrypt_aes_gcm_raw(
    ciphertext_and_tag: &[u8],
    key: &[u8],
    nonce: &[u8; AES_NONCE_LENGTH],
    aad: &[u8],
) -> Result<Zeroizing<Vec<u8>>, &'static CStr> {
    use aes_gcm::{
        Aes256Gcm, KeyInit,
        aead::{Aead, Nonce, Payload},
    };
    if key.len() != AES_KEY_LENGTH {
        return Err(c"ERR_CRYPTO_INVALID_KEY_SIZE");
    }
    let key = Zeroizing::new(key.to_vec());
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| c"ERR_CRYPTO_INVALID_KEY_SIZE")?;
    cipher
        .decrypt(
            &Nonce::<Aes256Gcm>::from(*nonce),
            Payload {
                msg: ciphertext_and_tag,
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| c"ERR_CRYPTO_DECRYPTION_FAILED")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_cipher_aes_encrypt(
    data_ptr: *const u8,
    data_len: usize,
    key_ptr: *const u8,
    key_len: usize,
    aad_ptr: *const u8,
    aad_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let (Ok(data), Ok(key), Ok(aad)) = (
            bytes_from_raw(data_ptr, data_len),
            bytes_from_raw(key_ptr, key_len),
            bytes_from_raw(aad_ptr, aad_len),
        ) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_INPUT", ());
        };
        if key.len() != AES_KEY_LENGTH {
            crypto_error!(c"ERR_CRYPTO_INVALID_KEY_SIZE", ());
        }
        let mut nonce_bytes = Zeroizing::new([0u8; AES_NONCE_LENGTH]);
        if fill_system_random(&mut *nonce_bytes).is_err() {
            crypto_error!(c"ERR_CRYPTO_RANDOM_FAILED", ());
        }
        let mut authenticated_data = Vec::with_capacity(AES_ENVELOPE_HEADER.len() + aad.len());
        authenticated_data.extend_from_slice(AES_ENVELOPE_HEADER);
        authenticated_data.extend_from_slice(&aad);
        let ciphertext_start = AES_ENVELOPE_HEADER.len() + AES_NONCE_LENGTH;
        let mut result = Vec::with_capacity(ciphertext_start + data.len() + AES_TAG_LENGTH);
        result.extend_from_slice(AES_ENVELOPE_HEADER);
        result.extend_from_slice(nonce_bytes.as_slice());
        result.extend_from_slice(&data);
        match encrypt_aes_gcm_in_place(
            &mut result[ciphertext_start..],
            &key,
            &nonce_bytes,
            &authenticated_data,
        ) {
            Ok(tag) => {
                result.extend_from_slice(&tag);
                write_array(output, &result);
            }
            Err(_) => {
                result.zeroize();
                crypto_error!(c"ERR_CRYPTO_ENCRYPTION_FAILED", ());
            }
        }
    }
}

pub(crate) fn decrypt_aes_bytes(
    envelope: &[u8],
    key: &[u8],
    aad: &[u8],
) -> Result<Zeroizing<Vec<u8>>, &'static CStr> {
    if key.len() != AES_KEY_LENGTH {
        return Err(c"ERR_CRYPTO_INVALID_KEY_SIZE");
    }
    let minimum = AES_ENVELOPE_HEADER.len() + AES_NONCE_LENGTH + AES_TAG_LENGTH;
    if envelope.len() < minimum || !envelope.starts_with(AES_ENVELOPE_HEADER) {
        return Err(c"ERR_CRYPTO_INVALID_ENVELOPE");
    }
    let nonce_start = AES_ENVELOPE_HEADER.len();
    let ciphertext_start = nonce_start + AES_NONCE_LENGTH;
    let nonce: &[u8; AES_NONCE_LENGTH] = envelope[nonce_start..ciphertext_start]
        .try_into()
        .map_err(|_| c"ERR_CRYPTO_INVALID_ENVELOPE")?;
    let mut authenticated_data = Vec::with_capacity(AES_ENVELOPE_HEADER.len() + aad.len());
    authenticated_data.extend_from_slice(AES_ENVELOPE_HEADER);
    authenticated_data.extend_from_slice(aad);
    decrypt_aes_gcm_raw(
        &envelope[ciphertext_start..],
        key,
        nonce,
        &authenticated_data,
    )
}

unsafe fn decrypt_aes(
    envelope_ptr: *const u8,
    envelope_len: usize,
    key_ptr: *const u8,
    key_len: usize,
    aad_ptr: *const u8,
    aad_len: usize,
) -> Result<Zeroizing<Vec<u8>>, &'static CStr> {
    unsafe {
        let (Ok(envelope), Ok(key), Ok(aad)) = (
            bytes_from_raw(envelope_ptr, envelope_len),
            bytes_from_raw(key_ptr, key_len),
            bytes_from_raw(aad_ptr, aad_len),
        ) else {
            return Err(c"ERR_CRYPTO_INVALID_INPUT");
        };
        decrypt_aes_bytes(&envelope, &key, &aad)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_cipher_aes_decrypt_str(
    envelope_ptr: *const u8,
    envelope_len: usize,
    key_ptr: *const u8,
    key_len: usize,
    aad_ptr: *const u8,
    aad_len: usize,
) -> TyrusString {
    unsafe {
        let plaintext = match decrypt_aes(
            envelope_ptr,
            envelope_len,
            key_ptr,
            key_len,
            aad_ptr,
            aad_len,
        ) {
            Ok(plaintext) => plaintext,
            Err(error) => crypto_error!(error, TyrusString::null()),
        };
        let Ok(text) = std::str::from_utf8(&plaintext) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_UTF8", TyrusString::null());
        };
        crate::strings::alloc_string(text)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_cipher_aes_decrypt_bytes(
    envelope_ptr: *const u8,
    envelope_len: usize,
    key_ptr: *const u8,
    key_len: usize,
    aad_ptr: *const u8,
    aad_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let plaintext = match decrypt_aes(
            envelope_ptr,
            envelope_len,
            key_ptr,
            key_len,
            aad_ptr,
            aad_len,
        ) {
            Ok(plaintext) => plaintext,
            Err(error) => crypto_error!(error, ()),
        };
        write_array(output, &plaintext);
    }
}

const ARGON2_PEPPER_MARKER: &[u8] = b"tyrus-pepper-v1";

#[derive(Clone, Copy)]
struct Argon2Profile {
    memory_cost: u32,
    time_cost: u32,
    parallelism: u32,
}

impl Default for Argon2Profile {
    fn default() -> Self {
        Self {
            memory_cost: ARGON2_MEMORY_COST,
            time_cost: ARGON2_TIME_COST,
            parallelism: ARGON2_PARALLELISM,
        }
    }
}

#[repr(C)]
pub struct TyrusArgon2HashOptions {
    memory_cost: *const u32,
    parallelism: *const u32,
    secret: *const TyrusArray,
    time_cost: *const u32,
    algorithm: *const TyrusString,
}

#[repr(C)]
pub struct TyrusArgon2VerifyOptions {
    secret: *const TyrusArray,
}

unsafe fn argon2_input(
    pointer: *const u8,
    length: usize,
    maximum: usize,
) -> Result<Zeroizing<Vec<u8>>, ()> {
    unsafe {
        if length > maximum {
            return Err(());
        }
        let bytes = bytes_from_raw(pointer, length)?;
        Ok(Zeroizing::new(bytes.to_vec()))
    }
}

fn hash_argon2id_with_salt(
    password: &[u8],
    pepper: Option<&[u8]>,
    salt_bytes: &[u8],
    profile: Argon2Profile,
) -> Result<String, &'static str> {
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::{AssociatedData, ParamsBuilder};
    let salt = SaltString::encode_b64(salt_bytes).map_err(|_| "ERR_CRYPTO_ARGON2_FAILED")?;
    let mut builder = ParamsBuilder::new();
    builder
        .m_cost(profile.memory_cost)
        .t_cost(profile.time_cost)
        .p_cost(profile.parallelism)
        .output_len(ARGON2_OUTPUT_LENGTH);
    if pepper.is_some() {
        builder.data(
            AssociatedData::new(ARGON2_PEPPER_MARKER)
                .map_err(|_| "ERR_CRYPTO_ARGON2_PARAMETERS")?,
        );
    }
    let params = builder
        .build()
        .map_err(|_| "ERR_CRYPTO_ARGON2_PARAMETERS")?;
    let a2 = match pepper {
        Some(secret) => argon2::Argon2::new_with_secret(
            secret,
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            params,
        )
        .map_err(|_| "ERR_CRYPTO_ARGON2_PARAMETERS")?,
        None => argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params),
    };
    a2.hash_password(password, &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| "ERR_CRYPTO_ARGON2_FAILED")
}

fn verify_argon2_password(
    hash: &str,
    password: &[u8],
    pepper: Option<&[u8]>,
) -> Result<bool, &'static str> {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    let parsed = PasswordHash::new(hash).map_err(|_| "ERR_CRYPTO_ARGON2_INVALID_HASH")?;
    let memory = parsed.params.get_decimal("m").unwrap_or(0);
    let iterations = parsed.params.get_decimal("t").unwrap_or(0);
    let parallelism = parsed.params.get_decimal("p").unwrap_or(0);
    if memory == 0
        || memory > 262_144
        || iterations == 0
        || iterations > 10
        || parallelism == 0
        || parallelism > 16
    {
        return Err("ERR_CRYPTO_ARGON2_PARAMETERS");
    }
    let params = argon2::Params::try_from(&parsed).map_err(|_| "ERR_CRYPTO_ARGON2_PARAMETERS")?;
    if params.data() == ARGON2_PEPPER_MARKER && pepper.is_none() {
        return Err("ERR_CRYPTO_ARGON2_PEPPER_REQUIRED");
    }
    let a2 = match pepper {
        Some(secret) => argon2::Argon2::new_with_secret(
            secret,
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::default(),
        )
        .map_err(|_| "ERR_CRYPTO_ARGON2_PARAMETERS")?,
        None => argon2::Argon2::default(),
    };
    Ok(a2.verify_password(password, &parsed).is_ok())
}

fn start_argon2_hash(
    password: Zeroizing<Vec<u8>>,
    pepper: Option<Zeroizing<Vec<u8>>>,
    profile: Argon2Profile,
) -> *mut promise::TyrusPromise {
    let Ok(permit) = argon2_slots().clone().try_acquire_owned() else {
        return unsafe { argon2_error_promise("ERR_CRYPTO_ARGON2_BUSY", true) };
    };
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let addr = promise as usize;
    executor().spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let mut salt_bytes = Zeroizing::new([0u8; ARGON2_SALT_LENGTH]);
            fill_system_random(&mut *salt_bytes).map_err(|()| "ERR_CRYPTO_RANDOM_FAILED")?;
            hash_argon2id_with_salt(
                &password,
                pepper.as_ref().map(|value| value.as_slice()),
                &*salt_bytes,
                profile,
            )
        })
        .await
        .map_err(|_| "ERR_CRYPTO_ARGON2_WORKER_FAILED")
        .and_then(|result| result);
        let p = addr as *mut promise::TyrusPromise;
        unsafe {
            fulfill_crypto_string_result(p, result.map_err(str::to_owned));
            crate::tyrus_promise_free(p);
        }
    });
    promise
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_argon2_hash(
    password_ptr: *const u8,
    password_len: usize,
    options: *const TyrusArgon2HashOptions,
) -> *mut promise::TyrusPromise {
    unsafe {
        let password = match argon2_input(password_ptr, password_len, MAX_PASSWORD_BYTES) {
            Ok(password) => password,
            Err(()) => {
                return argon2_error_promise("ERR_CRYPTO_ARGON2_INVALID_PASSWORD", true);
            }
        };
        let mut profile = Argon2Profile::default();
        let mut pepper = None;
        if let Some(options) = options.as_ref() {
            if let Some(value) = options.memory_cost.as_ref() {
                profile.memory_cost = *value;
            }
            if let Some(value) = options.time_cost.as_ref() {
                profile.time_cost = *value;
            }
            if let Some(value) = options.parallelism.as_ref() {
                profile.parallelism = *value;
            }
            if !(19_456..=262_144).contains(&profile.memory_cost)
                || !(2..=10).contains(&profile.time_cost)
                || !(1..=16).contains(&profile.parallelism)
            {
                return argon2_error_promise("ERR_CRYPTO_ARGON2_PARAMETERS", true);
            }
            if let Some(algorithm) = options.algorithm.as_ref() {
                let Ok(algorithm) = cstr_to_string(algorithm.borrow()) else {
                    return argon2_error_promise("ERR_CRYPTO_ARGON2_PARAMETERS", true);
                };
                if algorithm != "argon2id" {
                    return argon2_error_promise("ERR_CRYPTO_ARGON2_PARAMETERS", true);
                }
            }
            if let Some(secret) = options.secret.as_ref() {
                pepper = match argon2_input(secret.data, secret.length, MAX_PEPPER_BYTES) {
                    Ok(secret) if !secret.is_empty() => Some(secret),
                    _ => {
                        return argon2_error_promise("ERR_CRYPTO_ARGON2_INVALID_PEPPER", true);
                    }
                };
            }
        }
        start_argon2_hash(password, pepper, profile)
    }
}

fn start_argon2_verify(
    hash: String,
    password: Zeroizing<Vec<u8>>,
    pepper: Option<Zeroizing<Vec<u8>>>,
) -> *mut promise::TyrusPromise {
    let Ok(permit) = argon2_slots().clone().try_acquire_owned() else {
        return unsafe { argon2_error_promise("ERR_CRYPTO_ARGON2_BUSY", false) };
    };
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let addr = promise as usize;
    executor().spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            verify_argon2_password(
                &hash,
                &password,
                pepper.as_ref().map(|value| value.as_slice()),
            )
        })
        .await
        .map_err(|_| "ERR_CRYPTO_ARGON2_WORKER_FAILED")
        .and_then(|result| result);
        let p = addr as *mut promise::TyrusPromise;
        unsafe {
            fulfill_argon2_verify_result(p, result);
            crate::tyrus_promise_free(p);
        }
    });
    promise
}

unsafe fn read_argon2_hash(hash_ptr: TyrusString) -> Result<String, *mut promise::TyrusPromise> {
    unsafe {
        let hash = match cstr_to_string(hash_ptr) {
            Ok(s) if s.len() <= MAX_PHC_BYTES => s,
            _ => {
                return Err(argon2_error_promise(
                    "ERR_CRYPTO_ARGON2_INVALID_HASH",
                    false,
                ));
            }
        };
        Ok(hash)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_argon2_verify(
    hash_ptr: TyrusString,
    password_ptr: *const u8,
    password_len: usize,
    options: *const TyrusArgon2VerifyOptions,
) -> *mut promise::TyrusPromise {
    unsafe {
        let hash = match read_argon2_hash(hash_ptr) {
            Ok(hash) => hash,
            Err(promise) => return promise,
        };
        let password = match argon2_input(password_ptr, password_len, MAX_PASSWORD_BYTES) {
            Ok(password) => password,
            Err(()) => {
                return argon2_error_promise("ERR_CRYPTO_ARGON2_INVALID_PASSWORD", false);
            }
        };
        let mut pepper = None;
        if let Some(options) = options.as_ref()
            && let Some(secret) = options.secret.as_ref()
        {
            pepper = match argon2_input(secret.data, secret.length, MAX_PEPPER_BYTES) {
                Ok(secret) if !secret.is_empty() => Some(secret),
                _ => {
                    return argon2_error_promise("ERR_CRYPTO_ARGON2_INVALID_PEPPER", false);
                }
            };
        }
        start_argon2_verify(hash, password, pepper)
    }
}

pub(crate) fn argon2_hash_needs_rehash(hash: &str) -> Result<bool, ()> {
    use argon2::password_hash::PasswordHash;

    if hash.len() > MAX_PHC_BYTES {
        return Err(());
    }
    let parsed = PasswordHash::new(hash).map_err(|_| ())?;
    let salt = parsed.salt.ok_or(())?;
    let output = parsed.hash.as_ref().ok_or(())?;
    let mut decoded_salt = [0u8; 64];
    let salt_length = salt.decode_b64(&mut decoded_salt).map_err(|_| ())?.len();

    let parameter_count = parsed.params.iter().count();
    let marker_is_valid = match (parameter_count, parsed.params.get("data")) {
        (3, None) => true,
        (4, Some(value)) => {
            let mut marker = [0u8; 32];
            value
                .b64_decode(&mut marker)
                .is_ok_and(|decoded| decoded == ARGON2_PEPPER_MARKER)
        }
        _ => false,
    };

    Ok(parsed.algorithm.as_str() != "argon2id"
        || parsed.version != Some(0x13)
        || !matches!(parameter_count, 3 | 4)
        || !marker_is_valid
        || parsed.params.get_decimal("m") != Some(ARGON2_MEMORY_COST)
        || parsed.params.get_decimal("t") != Some(ARGON2_TIME_COST)
        || parsed.params.get_decimal("p") != Some(ARGON2_PARALLELISM)
        || salt_length != ARGON2_SALT_LENGTH
        || output.len() != ARGON2_OUTPUT_LENGTH)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_argon2_needs_rehash(hash_ptr: TyrusString) -> bool {
    unsafe {
        let Some(hash) = cstr_to_string(hash_ptr)
            .ok()
            .filter(|hash| hash.len() <= MAX_PHC_BYTES)
        else {
            crypto_error!(c"ERR_CRYPTO_ARGON2_INVALID_HASH", false);
        };
        match argon2_hash_needs_rehash(&hash) {
            Ok(result) => result,
            Err(()) => crypto_error!(c"ERR_CRYPTO_ARGON2_INVALID_HASH", false),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_argon2_needs_rehash_result(
    hash_ptr: TyrusString,
    out: *mut CryptoResultFfi<u8>,
) {
    tyrus_crypto_clear_error();
    let value = unsafe { tyrus_crypto_argon2_needs_rehash(hash_ptr) };
    unsafe { write_crypto_result(out, u8::from(value)) };
}

const MAX_JWKS_BYTES: usize = 256 * 1024;
const MAX_JWKS_KEYS: usize = 64;
const MAX_JWKS_CACHE_ENTRIES: usize = 32;
const JWKS_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

#[repr(C)]
pub struct TyrusJwtSignOptions {
    algorithm: *const TyrusString,
    audience: *const TyrusString,
    expires_in: *const TyrusString,
    issuer: *const TyrusString,
    jwt_id: *const TyrusString,
    key_id: *const TyrusString,
    no_timestamp: *const bool,
    subject: *const TyrusString,
}

#[repr(C)]
pub struct TyrusJwtVerifyOptions {
    algorithms: *const TyrusArray,
    audience: *const TyrusString,
    clock_tolerance: *const u32,
    issuer: *const TyrusString,
    max_age: *const TyrusString,
    subject: *const TyrusString,
}

#[repr(C)]
pub struct TyrusJwksFetchOptions {
    force_refresh: *const bool,
}

#[derive(Default)]
struct JwtSignOptions {
    algorithm: Option<jsonwebtoken::Algorithm>,
    audience: Option<String>,
    expires_in: Option<u64>,
    issuer: Option<String>,
    jwt_id: Option<String>,
    key_id: Option<String>,
    no_timestamp: bool,
    subject: Option<String>,
}

struct JwtVerifyOptions {
    algorithms: Vec<jsonwebtoken::Algorithm>,
    audience: Option<String>,
    clock_tolerance: u64,
    issuer: Option<String>,
    max_age: Option<u64>,
    subject: Option<String>,
}

impl Default for JwtVerifyOptions {
    fn default() -> Self {
        Self {
            algorithms: vec![jsonwebtoken::Algorithm::HS256],
            audience: None,
            clock_tolerance: 0,
            issuer: None,
            max_age: None,
            subject: None,
        }
    }
}

fn parse_jwt_algorithm(value: &str) -> Result<jsonwebtoken::Algorithm, &'static CStr> {
    use jsonwebtoken::Algorithm;
    match value {
        "HS256" => Ok(Algorithm::HS256),
        "HS384" => Ok(Algorithm::HS384),
        "HS512" => Ok(Algorithm::HS512),
        "RS256" => Ok(Algorithm::RS256),
        "RS384" => Ok(Algorithm::RS384),
        "RS512" => Ok(Algorithm::RS512),
        "ES256" => Ok(Algorithm::ES256),
        "ES384" => Ok(Algorithm::ES384),
        "EdDSA" => Ok(Algorithm::EdDSA),
        _ => Err(c"ERR_CRYPTO_JWT_INVALID_ALGORITHM"),
    }
}

fn parse_jwt_duration(value: &str) -> Result<u64, &'static CStr> {
    let (digits, multiplier) = match value.as_bytes().last().copied() {
        Some(b's') => (&value[..value.len() - 1], 1),
        Some(b'm') => (&value[..value.len() - 1], 60),
        Some(b'h') => (&value[..value.len() - 1], 60 * 60),
        Some(b'd') => (&value[..value.len() - 1], 24 * 60 * 60),
        _ => (value, 1),
    };
    let amount = digits
        .parse::<u64>()
        .ok()
        .filter(|amount| *amount > 0)
        .ok_or(c"ERR_CRYPTO_JWT_INVALID_CLAIMS")?;
    amount
        .checked_mul(multiplier)
        .filter(|seconds| *seconds <= 365 * 24 * 60 * 60)
        .ok_or(c"ERR_CRYPTO_JWT_INVALID_CLAIMS")
}

unsafe fn optional_ffi_string(value: *const TyrusString) -> Result<Option<String>, &'static CStr> {
    unsafe {
        let Some(value) = value.as_ref() else {
            return Ok(None);
        };
        cstr_to_string(value.borrow())
            .map(Some)
            .map_err(|()| c"ERR_CRYPTO_JWT_INVALID_OPTION")
    }
}

unsafe fn ffi_string_array(value: *const TyrusArray) -> Result<Vec<String>, &'static CStr> {
    unsafe {
        let Some(value) = value.as_ref() else {
            return Ok(Vec::new());
        };
        if value.length == 0 || value.length > 16 || value.data.is_null() {
            return Err(c"ERR_CRYPTO_JWT_INVALID_OPTION");
        }
        let stored = std::slice::from_raw_parts(value.data.cast::<TyrusString>(), value.length);
        stored
            .iter()
            .map(|text| {
                cstr_to_string(text.borrow()).map_err(|()| c"ERR_CRYPTO_JWT_INVALID_OPTION")
            })
            .collect()
    }
}

unsafe fn read_jwt_sign_options(
    options: *const TyrusJwtSignOptions,
) -> Result<JwtSignOptions, &'static CStr> {
    unsafe {
        let Some(options) = options.as_ref() else {
            return Ok(JwtSignOptions::default());
        };
        let options = JwtSignOptions {
            algorithm: optional_ffi_string(options.algorithm)?
                .as_deref()
                .map(parse_jwt_algorithm)
                .transpose()?,
            audience: optional_ffi_string(options.audience)?,
            expires_in: optional_ffi_string(options.expires_in)?
                .as_deref()
                .map(parse_jwt_duration)
                .transpose()?,
            issuer: optional_ffi_string(options.issuer)?,
            jwt_id: optional_ffi_string(options.jwt_id)?,
            key_id: optional_ffi_string(options.key_id)?,
            no_timestamp: options.no_timestamp.as_ref().copied().unwrap_or(false),
            subject: optional_ffi_string(options.subject)?,
        };
        for value in [
            options.audience.as_deref(),
            options.issuer.as_deref(),
            options.jwt_id.as_deref(),
            options.subject.as_deref(),
        ] {
            if value.is_some_and(|value| value.is_empty() || value.len() > 4096) {
                return Err(c"ERR_CRYPTO_JWT_INVALID_OPTION");
            }
        }
        if options
            .key_id
            .as_deref()
            .is_some_and(|value| value.is_empty() || value.len() > 256)
        {
            return Err(c"ERR_CRYPTO_JWT_INVALID_OPTION");
        }
        Ok(options)
    }
}

unsafe fn read_jwt_verify_options(
    options: *const TyrusJwtVerifyOptions,
) -> Result<JwtVerifyOptions, &'static CStr> {
    unsafe {
        let Some(options) = options.as_ref() else {
            return Ok(JwtVerifyOptions::default());
        };
        let algorithms = ffi_string_array(options.algorithms)?;
        let algorithms = if algorithms.is_empty() {
            vec![jsonwebtoken::Algorithm::HS256]
        } else {
            algorithms
                .iter()
                .map(|algorithm| parse_jwt_algorithm(algorithm))
                .collect::<Result<Vec<_>, _>>()?
        };
        let options = JwtVerifyOptions {
            algorithms,
            audience: optional_ffi_string(options.audience)?,
            clock_tolerance: options
                .clock_tolerance
                .as_ref()
                .copied()
                .unwrap_or(0)
                .into(),
            issuer: optional_ffi_string(options.issuer)?,
            max_age: optional_ffi_string(options.max_age)?
                .as_deref()
                .map(parse_jwt_duration)
                .transpose()?,
            subject: optional_ffi_string(options.subject)?,
        };
        for value in [
            options.audience.as_deref(),
            options.issuer.as_deref(),
            options.subject.as_deref(),
        ] {
            if value.is_some_and(|value| value.is_empty() || value.len() > 4096) {
                return Err(c"ERR_CRYPTO_JWT_INVALID_OPTION");
            }
        }
        if options.clock_tolerance > 24 * 60 * 60 {
            return Err(c"ERR_CRYPTO_JWT_INVALID_OPTION");
        }
        Ok(options)
    }
}

fn validate_jwt_secret(secret: &[u8]) -> Result<(), &'static CStr> {
    if secret.len() < 32 {
        Err(c"ERR_CRYPTO_JWT_WEAK_KEY")
    } else if secret.len() > MAX_JWT_SECRET_BYTES {
        Err(c"ERR_CRYPTO_JWT_INPUT_TOO_LARGE")
    } else {
        Ok(())
    }
}

fn jwt_encoding_key(
    algorithm: jsonwebtoken::Algorithm,
    key: &[u8],
) -> Result<jsonwebtoken::EncodingKey, &'static CStr> {
    if key.len() > MAX_JWT_SECRET_BYTES {
        return Err(c"ERR_CRYPTO_JWT_INPUT_TOO_LARGE");
    }
    use jsonwebtoken::Algorithm;
    match algorithm {
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
            validate_jwt_secret(key)?;
            Ok(jsonwebtoken::EncodingKey::from_secret(key))
        }
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
            jsonwebtoken::EncodingKey::from_rsa_pem(key).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_KEY")
        }
        Algorithm::ES256 | Algorithm::ES384 => {
            jsonwebtoken::EncodingKey::from_ec_pem(key).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_KEY")
        }
        Algorithm::EdDSA => {
            jsonwebtoken::EncodingKey::from_ed_pem(key).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_KEY")
        }
        _ => Err(c"ERR_CRYPTO_JWT_INVALID_ALGORITHM"),
    }
}

fn validate_jwk_for_verification(
    jwk: &jsonwebtoken::jwk::Jwk,
    algorithm: jsonwebtoken::Algorithm,
) -> Result<(), &'static CStr> {
    use jsonwebtoken::jwk::{AlgorithmParameters, KeyOperations, PublicKeyUse};
    if matches!(&jwk.algorithm, AlgorithmParameters::OctetKey(_)) {
        return Err(c"ERR_CRYPTO_JWT_INVALID_KEY");
    }
    if jwk.common.public_key_use.is_some() && jwk.common.key_operations.is_some() {
        return Err(c"ERR_CRYPTO_JWT_INVALID_KEY");
    }
    if !matches!(
        jwk.common.public_key_use,
        None | Some(PublicKeyUse::Signature)
    ) {
        return Err(c"ERR_CRYPTO_JWT_INVALID_KEY");
    }
    if jwk
        .common
        .key_operations
        .as_ref()
        .is_some_and(|operations| !operations.contains(&KeyOperations::Verify))
    {
        return Err(c"ERR_CRYPTO_JWT_INVALID_KEY");
    }
    if jwk
        .common
        .key_algorithm
        .is_some_and(|declared| declared.to_string() != format!("{algorithm:?}"))
    {
        return Err(c"ERR_CRYPTO_JWT_INVALID_ALGORITHM");
    }
    Ok(())
}

fn jwk_contains_private_material(value: &serde_json::Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    ["d", "p", "q", "dp", "dq", "qi", "oth"]
        .iter()
        .any(|parameter| object.contains_key(*parameter))
}

fn jwks_contains_private_material(value: &serde_json::Value) -> bool {
    value
        .get("keys")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|keys| keys.iter().any(jwk_contains_private_material))
}

fn jwt_decoding_key(
    algorithm: jsonwebtoken::Algorithm,
    key: &[u8],
) -> Result<jsonwebtoken::DecodingKey, &'static CStr> {
    if key.len() > MAX_JWT_SECRET_BYTES {
        return Err(c"ERR_CRYPTO_JWT_INPUT_TOO_LARGE");
    }
    use jsonwebtoken::Algorithm;
    if key.first() == Some(&b'{') {
        let value: serde_json::Value =
            serde_json::from_slice(key).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_KEY")?;
        if jwk_contains_private_material(&value) {
            return Err(c"ERR_CRYPTO_JWT_INVALID_KEY");
        }
        let jwk: jsonwebtoken::jwk::Jwk =
            serde_json::from_value(value).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_KEY")?;
        validate_jwk_for_verification(&jwk, algorithm)?;
        let decoding =
            jsonwebtoken::DecodingKey::from_jwk(&jwk).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_KEY")?;
        if decoding.family() != algorithm.family() {
            return Err(c"ERR_CRYPTO_JWT_INVALID_ALGORITHM");
        }
        return Ok(decoding);
    }
    match algorithm {
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
            validate_jwt_secret(key)?;
            Ok(jsonwebtoken::DecodingKey::from_secret(key))
        }
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
            jsonwebtoken::DecodingKey::from_rsa_pem(key).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_KEY")
        }
        Algorithm::ES256 | Algorithm::ES384 => {
            jsonwebtoken::DecodingKey::from_ec_pem(key).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_KEY")
        }
        Algorithm::EdDSA => {
            jsonwebtoken::DecodingKey::from_ed_pem(key).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_KEY")
        }
        _ => Err(c"ERR_CRYPTO_JWT_INVALID_ALGORITHM"),
    }
}

fn sign_jwt(
    payload: &str,
    key: &[u8],
    issued_at: u64,
    options: &JwtSignOptions,
) -> Result<String, &'static CStr> {
    if payload.len() > MAX_JWT_BYTES {
        return Err(c"ERR_CRYPTO_JWT_INPUT_TOO_LARGE");
    }
    let mut claims: serde_json::Value =
        serde_json::from_str(payload).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_PAYLOAD")?;
    if !claims.is_object() {
        return Err(c"ERR_CRYPTO_JWT_INVALID_PAYLOAD");
    }
    if !options.no_timestamp && claims.get("iat").is_none() {
        claims["iat"] = serde_json::Value::Number(issued_at.into());
    }
    if options.no_timestamp
        && let Some(claims) = claims.as_object_mut()
    {
        claims.remove("iat");
    }
    if let Some(expires_in) = options.expires_in {
        let expiration = issued_at
            .checked_add(expires_in)
            .ok_or(c"ERR_CRYPTO_JWT_INVALID_CLAIMS")?;
        claims["exp"] = serde_json::Value::Number(expiration.into());
    }
    for (name, value) in [
        ("aud", options.audience.as_ref()),
        ("iss", options.issuer.as_ref()),
        ("jti", options.jwt_id.as_ref()),
        ("sub", options.subject.as_ref()),
    ] {
        if let Some(value) = value {
            claims[name] = serde_json::Value::String(value.clone());
        }
    }
    let algorithm = options.algorithm.unwrap_or(jsonwebtoken::Algorithm::HS256);
    let mut header = jsonwebtoken::Header::new(algorithm);
    header.kid.clone_from(&options.key_id);
    let encoding = jwt_encoding_key(algorithm, key)?;
    let token = jsonwebtoken::encode(&header, &claims, &encoding)
        .map_err(|_| c"ERR_CRYPTO_JWT_SIGN_FAILED")?;
    if token.len() > MAX_JWT_BYTES {
        return Err(c"ERR_CRYPTO_JWT_INPUT_TOO_LARGE");
    }
    Ok(token)
}

fn jwt_validation(options: &JwtVerifyOptions) -> jsonwebtoken::Validation {
    let mut validation = jsonwebtoken::Validation::new(options.algorithms[0]);
    validation.algorithms.clone_from(&options.algorithms);
    validation.required_spec_claims.clear();
    validation.leeway = options.clock_tolerance;
    validation.validate_nbf = true;
    if let Some(audience) = &options.audience {
        validation.set_audience(&[audience]);
    } else {
        validation.validate_aud = false;
    }
    if let Some(issuer) = &options.issuer {
        validation.set_issuer(&[issuer]);
    }
    validation.sub.clone_from(&options.subject);
    validation
}

fn validate_jwt_header(header: &jsonwebtoken::Header) -> Result<(), &'static CStr> {
    if header.crit.is_some() {
        return Err(c"ERR_CRYPTO_JWT_UNSUPPORTED_CRITICAL_HEADER");
    }
    Ok(())
}

fn map_jwt_decode_error(error: &jsonwebtoken::errors::Error) -> &'static CStr {
    use jsonwebtoken::errors::ErrorKind;
    match error.kind() {
        ErrorKind::InvalidSignature => c"ERR_CRYPTO_JWT_INVALID_SIGNATURE",
        ErrorKind::ExpiredSignature => c"ERR_CRYPTO_JWT_EXPIRED",
        ErrorKind::InvalidAlgorithm | ErrorKind::MissingAlgorithm => {
            c"ERR_CRYPTO_JWT_INVALID_ALGORITHM"
        }
        ErrorKind::MissingRequiredClaim(_)
        | ErrorKind::InvalidClaimFormat(_)
        | ErrorKind::InvalidIssuer
        | ErrorKind::InvalidAudience
        | ErrorKind::InvalidSubject
        | ErrorKind::ImmatureSignature => c"ERR_CRYPTO_JWT_INVALID_CLAIMS",
        _ => c"ERR_CRYPTO_JWT_MALFORMED",
    }
}

fn validate_jwt_max_age(
    claims: &serde_json::Value,
    options: &JwtVerifyOptions,
    now: u64,
) -> Result<(), &'static CStr> {
    let Some(max_age) = options.max_age else {
        return Ok(());
    };
    let issued_at = claims
        .get("iat")
        .and_then(serde_json::Value::as_u64)
        .ok_or(c"ERR_CRYPTO_JWT_INVALID_CLAIMS")?;
    if issued_at > now.saturating_add(options.clock_tolerance)
        || now.saturating_sub(issued_at) > max_age.saturating_add(options.clock_tolerance)
    {
        return Err(c"ERR_CRYPTO_JWT_INVALID_CLAIMS");
    }
    Ok(())
}

fn validate_jwt_claim_constraints(
    claims: &serde_json::Value,
    options: &JwtVerifyOptions,
) -> Result<(), &'static CStr> {
    if let Some(expected) = &options.audience {
        let matches = match claims.get("aud") {
            Some(serde_json::Value::String(actual)) => actual == expected,
            Some(serde_json::Value::Array(actual)) => actual
                .iter()
                .any(|value| value.as_str() == Some(expected.as_str())),
            _ => false,
        };
        if !matches {
            return Err(c"ERR_CRYPTO_JWT_INVALID_CLAIMS");
        }
    }
    for (name, expected) in [
        ("iss", options.issuer.as_deref()),
        ("sub", options.subject.as_deref()),
    ] {
        if expected.is_some_and(|expected| {
            claims.get(name).and_then(serde_json::Value::as_str) != Some(expected)
        }) {
            return Err(c"ERR_CRYPTO_JWT_INVALID_CLAIMS");
        }
    }
    Ok(())
}

fn verify_jwt(
    token: &str,
    key: &[u8],
    options: &JwtVerifyOptions,
) -> Result<String, &'static CStr> {
    if token.len() > MAX_JWT_BYTES {
        return Err(c"ERR_CRYPTO_JWT_INPUT_TOO_LARGE");
    }
    let header = jsonwebtoken::decode_header(token).map_err(|_| c"ERR_CRYPTO_JWT_MALFORMED")?;
    validate_jwt_header(&header)?;
    if !options.algorithms.contains(&header.alg) {
        return Err(c"ERR_CRYPTO_JWT_INVALID_ALGORITHM");
    }
    let decoding = jwt_decoding_key(header.alg, key)?;
    let validation = jwt_validation(options);
    match jsonwebtoken::decode::<serde_json::Value>(token, &decoding, &validation) {
        Ok(data) if data.claims.is_object() => {
            validate_jwt_claim_constraints(&data.claims, options)?;
            validate_jwt_max_age(&data.claims, options, jsonwebtoken::get_current_timestamp())?;
            serde_json::to_string(&data.claims).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_CLAIMS")
        }
        Ok(_) => Err(c"ERR_CRYPTO_JWT_INVALID_CLAIMS"),
        Err(error) => Err(map_jwt_decode_error(&error)),
    }
}

fn verify_jwt_with_jwks(
    token: &str,
    jwks_json: &str,
    options: &JwtVerifyOptions,
) -> Result<String, &'static CStr> {
    if jwks_json.len() > MAX_JWKS_BYTES {
        return Err(c"ERR_CRYPTO_JWKS_INVALID");
    }
    let header = jsonwebtoken::decode_header(token).map_err(|_| c"ERR_CRYPTO_JWT_MALFORMED")?;
    validate_jwt_header(&header)?;
    if !options.algorithms.contains(&header.alg) {
        return Err(c"ERR_CRYPTO_JWT_INVALID_ALGORITHM");
    }
    let kid = header
        .kid
        .filter(|kid| !kid.is_empty() && kid.len() <= 256)
        .ok_or(c"ERR_CRYPTO_JWT_MISSING_KID")?;
    let value: serde_json::Value =
        serde_json::from_str(jwks_json).map_err(|_| c"ERR_CRYPTO_JWKS_INVALID")?;
    if jwks_contains_private_material(&value) {
        return Err(c"ERR_CRYPTO_JWT_INVALID_KEY");
    }
    let set: jsonwebtoken::jwk::JwkSet =
        serde_json::from_value(value).map_err(|_| c"ERR_CRYPTO_JWKS_INVALID")?;
    if set.keys.is_empty() || set.keys.len() > MAX_JWKS_KEYS {
        return Err(c"ERR_CRYPTO_JWKS_INVALID");
    }
    let mut matches = set
        .keys
        .iter()
        .filter(|jwk| jwk.common.key_id.as_deref() == Some(kid.as_str()));
    let jwk = matches.next().ok_or(c"ERR_CRYPTO_JWKS_KEY_NOT_FOUND")?;
    if matches.next().is_some() {
        return Err(c"ERR_CRYPTO_JWKS_DUPLICATE_KID");
    }
    validate_jwk_for_verification(jwk, header.alg)?;
    let decoding =
        jsonwebtoken::DecodingKey::from_jwk(jwk).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_KEY")?;
    if decoding.family() != header.alg.family() {
        return Err(c"ERR_CRYPTO_JWT_INVALID_ALGORITHM");
    }
    let validation = jwt_validation(options);
    match jsonwebtoken::decode::<serde_json::Value>(token, &decoding, &validation) {
        Ok(data) if data.claims.is_object() => {
            validate_jwt_claim_constraints(&data.claims, options)?;
            validate_jwt_max_age(&data.claims, options, jsonwebtoken::get_current_timestamp())?;
            serde_json::to_string(&data.claims).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_CLAIMS")
        }
        Ok(_) => Err(c"ERR_CRYPTO_JWT_INVALID_CLAIMS"),
        Err(error) => Err(map_jwt_decode_error(&error)),
    }
}

type JwksFetchLocks = HashMap<String, (Instant, Arc<tokio::sync::Mutex<()>>)>;

fn jwks_fetch_locks() -> &'static Mutex<JwksFetchLocks> {
    static LOCKS: OnceLock<Mutex<JwksFetchLocks>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn jwks_url_lock(url: &str) -> Result<Arc<tokio::sync::Mutex<()>>, String> {
    let mut locks = jwks_fetch_locks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((used, lock)) = locks.get_mut(url) {
        *used = Instant::now();
        return Ok(lock.clone());
    }
    if locks.len() >= MAX_JWKS_CACHE_ENTRIES {
        let idle = locks
            .iter()
            .filter(|(_, (_, lock))| Arc::strong_count(lock) == 1)
            .min_by_key(|(_, (used, _))| *used)
            .map(|(url, _)| url.clone())
            .ok_or_else(|| "ERR_CRYPTO_JWKS_BUSY".to_string())?;
        locks.remove(&idle);
    }
    let lock = Arc::new(tokio::sync::Mutex::new(()));
    locks.insert(url.to_string(), (Instant::now(), lock.clone()));
    Ok(lock)
}

fn jwks_cache() -> &'static Mutex<HashMap<String, (Instant, String)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (Instant, String)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached_jwks(url: &str, force_refresh: bool) -> Option<String> {
    if force_refresh {
        return None;
    }
    jwks_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(url)
        .filter(|(created, _)| created.elapsed() < JWKS_CACHE_TTL)
        .map(|(_, value)| value.clone())
}

fn refreshed_jwks_since(url: &str, request_started: Instant) -> Option<String> {
    jwks_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(url)
        .filter(|(created, _)| *created >= request_started)
        .map(|(_, value)| value.clone())
}

fn is_forbidden_ipv4(address: Ipv4Addr) -> bool {
    let [a, b, c, _] = address.octets();
    address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_multicast()
        || address.is_broadcast()
        || address.is_unspecified()
        || address.is_documentation()
        || a == 0
        || a >= 240
        || (a == 100 && (64..=127).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 88 && c == 99)
        || (a == 198 && (b == 18 || b == 19))
}

fn is_forbidden_ipv6(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    address.is_loopback()
        || address.is_unspecified()
        || address.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || address.to_ipv4_mapped().is_some_and(is_forbidden_ipv4)
}

fn is_forbidden_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_forbidden_ipv4(address),
        IpAddr::V6(address) => is_forbidden_ipv6(address),
    }
}

pub(crate) fn validate_fetched_jwks(value: &str) -> Result<(), String> {
    use jsonwebtoken::jwk::{AlgorithmParameters, KeyOperations, PublicKeyUse};
    let raw: serde_json::Value =
        serde_json::from_str(value).map_err(|_| "ERR_CRYPTO_JWKS_INVALID".to_string())?;
    if jwks_contains_private_material(&raw) {
        return Err("ERR_CRYPTO_JWKS_INVALID".to_string());
    }
    let set: jsonwebtoken::jwk::JwkSet =
        serde_json::from_value(raw).map_err(|_| "ERR_CRYPTO_JWKS_INVALID".to_string())?;
    if set.keys.is_empty() || set.keys.len() > MAX_JWKS_KEYS {
        return Err("ERR_CRYPTO_JWKS_INVALID".to_string());
    }
    let mut key_ids = HashSet::with_capacity(set.keys.len());
    for key in &set.keys {
        if matches!(&key.algorithm, AlgorithmParameters::OctetKey(_)) {
            return Err("ERR_CRYPTO_JWKS_INVALID".to_string());
        }
        if key.common.public_key_use.is_some() && key.common.key_operations.is_some() {
            return Err("ERR_CRYPTO_JWKS_INVALID".to_string());
        }
        if !matches!(
            key.common.public_key_use,
            None | Some(PublicKeyUse::Signature)
        ) || key
            .common
            .key_operations
            .as_ref()
            .is_some_and(|operations| !operations.contains(&KeyOperations::Verify))
        {
            return Err("ERR_CRYPTO_JWKS_INVALID".to_string());
        }
        let key_id = key
            .common
            .key_id
            .as_deref()
            .filter(|key_id| !key_id.is_empty() && key_id.len() <= 256)
            .ok_or_else(|| "ERR_CRYPTO_JWKS_INVALID".to_string())?;
        if !key_ids.insert(key_id) {
            return Err("ERR_CRYPTO_JWKS_DUPLICATE_KID".to_string());
        }
    }
    Ok(())
}

async fn fetch_jwks(url: String, force_refresh: bool) -> Result<String, String> {
    let parsed =
        reqwest::Url::parse(&url).map_err(|_| "ERR_CRYPTO_JWKS_INVALID_URL".to_string())?;
    if parsed.scheme() != "https"
        || parsed.port_or_known_default() != Some(443)
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
    {
        return Err("ERR_CRYPTO_JWKS_INVALID_URL".to_string());
    }
    let host = parsed
        .host_str()
        .filter(|host| !host.eq_ignore_ascii_case("localhost"))
        .ok_or_else(|| "ERR_CRYPTO_JWKS_INVALID_URL".to_string())?
        .to_ascii_lowercase();
    if host.parse::<IpAddr>().is_ok() {
        return Err("ERR_CRYPTO_JWKS_FORBIDDEN_ADDRESS".to_string());
    }

    let request_started = Instant::now();
    let url_lock = jwks_url_lock(&url)?;
    let _single_flight = url_lock.lock().await;
    if force_refresh && let Some(value) = refreshed_jwks_since(&url, request_started) {
        return Ok(value);
    }
    if let Some(value) = cached_jwks(&url, force_refresh) {
        return Ok(value);
    }

    let addresses = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::lookup_host((host.as_str(), 443)),
    )
    .await
    .map_err(|_| "ERR_CRYPTO_JWKS_TIMEOUT".to_string())?
    .map_err(|_| "ERR_CRYPTO_JWKS_DNS_FAILED".to_string())?
    .collect::<Vec<_>>();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| is_forbidden_ip(address.ip()))
    {
        return Err("ERR_CRYPTO_JWKS_FORBIDDEN_ADDRESS".to_string());
    }
    let client = reqwest::Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(8))
        .resolve_to_addrs(&host, &addresses)
        .build()
        .map_err(|_| "ERR_CRYPTO_JWKS_FETCH_FAILED".to_string())?;
    let response = client.get(parsed).send().await.map_err(|error| {
        if error.is_timeout() {
            "ERR_CRYPTO_JWKS_TIMEOUT".to_string()
        } else {
            "ERR_CRYPTO_JWKS_FETCH_FAILED".to_string()
        }
    })?;
    if !response.status().is_success() {
        return Err("ERR_CRYPTO_JWKS_HTTP_STATUS".to_string());
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_JWKS_BYTES as u64)
    {
        return Err("ERR_CRYPTO_JWKS_TOO_LARGE".to_string());
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "ERR_CRYPTO_JWKS_FETCH_FAILED".to_string())?;
        if body.len().saturating_add(chunk.len()) > MAX_JWKS_BYTES {
            return Err("ERR_CRYPTO_JWKS_TOO_LARGE".to_string());
        }
        body.extend_from_slice(&chunk);
    }
    let value = String::from_utf8(body).map_err(|_| "ERR_CRYPTO_JWKS_INVALID".to_string())?;
    validate_fetched_jwks(&value)?;

    let mut cache = jwks_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.retain(|_, (created, _)| created.elapsed() < JWKS_CACHE_TTL);
    if cache.len() >= MAX_JWKS_CACHE_ENTRIES
        && let Some(oldest) = cache
            .iter()
            .min_by_key(|(_, (created, _))| *created)
            .map(|(key, _)| key.clone())
    {
        cache.remove(&oldest);
    }
    cache.insert(url, (Instant::now(), value.clone()));
    Ok(value)
}

fn start_jwks_fetch(url: String, force_refresh: bool) -> *mut promise::TyrusPromise {
    let promise = crate::tyrus_promise_pending();
    unsafe { crate::tyrus_promise_retain(promise) };
    let address = promise as usize;
    executor().spawn(async move {
        let result = fetch_jwks(url, force_refresh).await;
        let promise = address as *mut promise::TyrusPromise;
        unsafe { fulfill_crypto_string_result(promise, result) };
        unsafe { crate::tyrus_promise_free(promise) };
    });
    promise
}

fn decode_jwt_unverified(token: &str) -> Result<String, &'static CStr> {
    if token.len() > MAX_JWT_BYTES {
        return Err(c"ERR_CRYPTO_JWT_INPUT_TOO_LARGE");
    }
    match jsonwebtoken::dangerous::insecure_decode::<serde_json::Value>(token) {
        Ok(data) if data.claims.is_object() => {
            serde_json::to_string(&data.claims).map_err(|_| c"ERR_CRYPTO_JWT_INVALID_CLAIMS")
        }
        Ok(_) => Err(c"ERR_CRYPTO_JWT_INVALID_CLAIMS"),
        Err(_) => Err(c"ERR_CRYPTO_JWT_MALFORMED"),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_jwt_sign(
    payload_ptr: TyrusString,
    key_ptr: TyrusString,
    options_ptr: *const TyrusJwtSignOptions,
) -> TyrusString {
    unsafe {
        let Ok(payload_str) = cstr_to_string(payload_ptr) else {
            crypto_error!(c"ERR_CRYPTO_JWT_INVALID_PAYLOAD", TyrusString::null());
        };
        let Ok(key) = cstr_to_string(key_ptr) else {
            crypto_error!(c"ERR_CRYPTO_JWT_INVALID_KEY", TyrusString::null());
        };
        let key = Zeroizing::new(key);
        let options = match read_jwt_sign_options(options_ptr) {
            Ok(options) => options,
            Err(error) => crypto_error!(error, TyrusString::null()),
        };
        let Ok(elapsed) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
            crypto_error!(c"ERR_CRYPTO_CLOCK_FAILED", TyrusString::null());
        };
        let token = match sign_jwt(&payload_str, key.as_bytes(), elapsed.as_secs(), &options) {
            Ok(token) => token,
            Err(error) => crypto_error!(error, TyrusString::null()),
        };
        crate::strings::alloc_string(&token)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_jwt_sign_result(
    payload_ptr: TyrusString,
    key_ptr: TyrusString,
    options_ptr: *const TyrusJwtSignOptions,
    out: *mut CryptoResultFfi<TyrusString>,
) {
    tyrus_crypto_clear_error();
    let value = unsafe { tyrus_crypto_jwt_sign(payload_ptr, key_ptr, options_ptr) };
    unsafe { write_crypto_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_jwt_verify(
    token_ptr: TyrusString,
    key_ptr: TyrusString,
    options_ptr: *const TyrusJwtVerifyOptions,
) -> TyrusString {
    unsafe {
        let Ok(token) = cstr_to_string(token_ptr) else {
            crypto_error!(c"ERR_CRYPTO_JWT_MALFORMED", TyrusString::null());
        };
        let Ok(key) = cstr_to_string(key_ptr) else {
            crypto_error!(c"ERR_CRYPTO_JWT_INVALID_KEY", TyrusString::null());
        };
        let key = Zeroizing::new(key);
        let options = match read_jwt_verify_options(options_ptr) {
            Ok(options) => options,
            Err(error) => crypto_error!(error, TyrusString::null()),
        };
        let result = match verify_jwt(&token, key.as_bytes(), &options) {
            Ok(result) => result,
            Err(error) => crypto_error!(error, TyrusString::null()),
        };
        crate::strings::alloc_string(&result)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_jwt_verify_result(
    token_ptr: TyrusString,
    key_ptr: TyrusString,
    options_ptr: *const TyrusJwtVerifyOptions,
    out: *mut CryptoResultFfi<TyrusString>,
) {
    tyrus_crypto_clear_error();
    let value = unsafe { tyrus_crypto_jwt_verify(token_ptr, key_ptr, options_ptr) };
    unsafe { write_crypto_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_jwt_verify_with_jwks(
    token_ptr: TyrusString,
    jwks_ptr: TyrusString,
    options_ptr: *const TyrusJwtVerifyOptions,
) -> TyrusString {
    unsafe {
        let Ok(token) = cstr_to_string(token_ptr) else {
            crypto_error!(c"ERR_CRYPTO_JWT_MALFORMED", TyrusString::null());
        };
        let Ok(jwks) = cstr_to_string(jwks_ptr) else {
            crypto_error!(c"ERR_CRYPTO_JWKS_INVALID", TyrusString::null());
        };
        let options = match read_jwt_verify_options(options_ptr) {
            Ok(options) => options,
            Err(error) => crypto_error!(error, TyrusString::null()),
        };
        let result = match verify_jwt_with_jwks(&token, &jwks, &options) {
            Ok(result) => result,
            Err(error) => crypto_error!(error, TyrusString::null()),
        };
        crate::strings::alloc_string(&result)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_jwt_verify_with_jwks_result(
    token_ptr: TyrusString,
    jwks_ptr: TyrusString,
    options_ptr: *const TyrusJwtVerifyOptions,
    out: *mut CryptoResultFfi<TyrusString>,
) {
    tyrus_crypto_clear_error();
    let value = unsafe { tyrus_crypto_jwt_verify_with_jwks(token_ptr, jwks_ptr, options_ptr) };
    unsafe { write_crypto_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_jwt_fetch_jwks(
    url_ptr: TyrusString,
    options_ptr: *const TyrusJwksFetchOptions,
) -> *mut promise::TyrusPromise {
    unsafe {
        let Ok(url) = cstr_to_string(url_ptr) else {
            return crypto_string_error_promise("ERR_CRYPTO_JWKS_INVALID_URL");
        };
        if url.len() > 2048 {
            return crypto_string_error_promise("ERR_CRYPTO_JWKS_INVALID_URL");
        }
        let force_refresh = options_ptr
            .as_ref()
            .and_then(|options| options.force_refresh.as_ref())
            .copied()
            .unwrap_or(false);
        start_jwks_fetch(url, force_refresh)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_jwt_decode_unverified(token_ptr: TyrusString) -> TyrusString {
    unsafe {
        let Ok(token) = cstr_to_string(token_ptr) else {
            crypto_error!(c"ERR_CRYPTO_JWT_MALFORMED", TyrusString::null());
        };
        let result = match decode_jwt_unverified(&token) {
            Ok(result) => result,
            Err(error) => crypto_error!(error, TyrusString::null()),
        };
        crate::strings::alloc_string(&result)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_jwt_decode_unverified_result(
    token_ptr: TyrusString,
    out: *mut CryptoResultFfi<TyrusString>,
) {
    tyrus_crypto_clear_error();
    let value = unsafe { tyrus_crypto_jwt_decode_unverified(token_ptr) };
    unsafe { write_crypto_result(out, value) };
}

enum DigestStreamInner {
    Sha256(sha2::Sha256),
    Sha384(sha2::Sha384),
    Sha512(sha2::Sha512),
}

pub struct DigestStream {
    inner: DigestStreamInner,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_create(
    algorithm_ptr: TyrusString,
) -> *mut DigestStream {
    unsafe {
        let Ok(alg) = cstr_to_string(algorithm_ptr) else {
            crypto_error!(c"ERR_CRYPTO_DIGEST_ALGORITHM", ptr::null_mut());
        };
        let inner = match alg.to_lowercase().as_str() {
            "sha256" => DigestStreamInner::Sha256(sha2::Sha256::new()),
            "sha384" => DigestStreamInner::Sha384(sha2::Sha384::new()),
            "sha512" => DigestStreamInner::Sha512(sha2::Sha512::new()),
            _ => crypto_error!(c"ERR_CRYPTO_DIGEST_ALGORITHM", ptr::null_mut()),
        };
        Box::into_raw(Box::new(DigestStream { inner }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_create_result(
    algorithm_ptr: TyrusString,
    out: *mut CryptoResultFfi<*mut DigestStream>,
) {
    tyrus_crypto_clear_error();
    let stream = unsafe { tyrus_crypto_digest_create(algorithm_ptr) };
    unsafe { write_crypto_result(out, stream) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_stream_update(
    stream: *mut DigestStream,
    data_ptr: *const u8,
    data_len: usize,
) {
    unsafe {
        if stream.is_null() {
            crypto_error!(c"ERR_CRYPTO_INVALID_STREAM", ());
        }
        let Ok(data) = bytes_from_raw(data_ptr, data_len) else {
            crypto_error!(c"ERR_CRYPTO_INVALID_INPUT", ());
        };
        let s = &mut *stream;
        match &mut s.inner {
            DigestStreamInner::Sha256(h) => h.update(data),
            DigestStreamInner::Sha384(h) => h.update(data),
            DigestStreamInner::Sha512(h) => h.update(data),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_stream_update_result(
    stream: *mut DigestStream,
    data_ptr: *const u8,
    data_len: usize,
    out: *mut CryptoResultFfi<*mut DigestStream>,
) {
    tyrus_crypto_clear_error();
    unsafe { tyrus_crypto_digest_stream_update(stream, data_ptr, data_len) };
    unsafe { write_crypto_result(out, stream) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_stream_digest(
    stream: *mut DigestStream,
    output: *mut TyrusArray,
) {
    unsafe {
        if stream.is_null() {
            crypto_error!(c"ERR_CRYPTO_INVALID_STREAM", ());
        }
        let s = &mut *stream;
        let hash = match &s.inner {
            DigestStreamInner::Sha256(h) => h.clone().finalize().to_vec(),
            DigestStreamInner::Sha384(h) => h.clone().finalize().to_vec(),
            DigestStreamInner::Sha512(h) => h.clone().finalize().to_vec(),
        };
        write_array(output, &hash);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_stream_digest_result(
    stream: *mut DigestStream,
    out: *mut CryptoResultFfi<TyrusArray>,
) {
    tyrus_crypto_clear_error();
    let mut value = TyrusArray {
        data: ptr::null_mut(),
        length: 0,
        capacity: 0,
    };
    unsafe { tyrus_crypto_digest_stream_digest(stream, &mut value) };
    unsafe { write_crypto_result(out, value) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_stream_reset(stream: *mut DigestStream) {
    unsafe {
        if stream.is_null() {
            crypto_error!(c"ERR_CRYPTO_INVALID_STREAM", ());
        }
        let s = &mut *stream;
        match &mut s.inner {
            DigestStreamInner::Sha256(h) => *h = sha2::Sha256::new(),
            DigestStreamInner::Sha384(h) => *h = sha2::Sha384::new(),
            DigestStreamInner::Sha512(h) => *h = sha2::Sha512::new(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_stream_reset_result(
    stream: *mut DigestStream,
    out: *mut CryptoResultFfi<*mut DigestStream>,
) {
    tyrus_crypto_clear_error();
    unsafe { tyrus_crypto_digest_stream_reset(stream) };
    unsafe { write_crypto_result(out, stream) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_crypto_digest_stream_free(stream: *mut DigestStream) {
    unsafe {
        if !stream.is_null() {
            drop(Box::from_raw(stream));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_stream_result_wrappers_return_structured_errors() {
        let mut create = std::mem::MaybeUninit::<CryptoResultFfi<*mut DigestStream>>::uninit();
        unsafe {
            tyrus_crypto_digest_create_result(
                TyrusString::literal("unsupported"),
                create.as_mut_ptr(),
            )
        };
        let create = unsafe { create.assume_init() };
        assert_eq!(create.tag, 1);
        let error = unsafe { &*create.payload.err };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_CRYPTO_DIGEST_ALGORITHM"
        );

        let mut update = std::mem::MaybeUninit::<CryptoResultFfi<*mut DigestStream>>::uninit();
        unsafe {
            tyrus_crypto_digest_stream_update_result(
                ptr::null_mut(),
                ptr::null(),
                0,
                update.as_mut_ptr(),
            )
        };
        let update = unsafe { update.assume_init() };
        assert_eq!(update.tag, 1);
        let error = unsafe { &*update.payload.err };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_CRYPTO_INVALID_STREAM");

        let mut digest = std::mem::MaybeUninit::<CryptoResultFfi<TyrusArray>>::uninit();
        unsafe { tyrus_crypto_digest_stream_digest_result(ptr::null_mut(), digest.as_mut_ptr()) };
        let digest = unsafe { digest.assume_init() };
        assert_eq!(digest.tag, 1);
        let error = unsafe { &*digest.payload.err };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_CRYPTO_INVALID_STREAM");

        let mut reset = std::mem::MaybeUninit::<CryptoResultFfi<*mut DigestStream>>::uninit();
        unsafe { tyrus_crypto_digest_stream_reset_result(ptr::null_mut(), reset.as_mut_ptr()) };
        let reset = unsafe { reset.assume_init() };
        assert_eq!(reset.tag, 1);
        let error = unsafe { &*reset.payload.err };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_CRYPTO_INVALID_STREAM");
    }

    #[test]
    fn random_int_result_wrapper_returns_ok_and_structured_error() {
        let mut ok = std::mem::MaybeUninit::<CryptoResultFfi<i64>>::uninit();
        unsafe { tyrus_crypto_rand_random_int_result(-4, -4, ok.as_mut_ptr()) };
        let ok = unsafe { ok.assume_init() };
        assert_eq!(ok.tag, 0);
        assert_eq!(unsafe { *ok.payload.ok }, -4);

        let mut err = std::mem::MaybeUninit::<CryptoResultFfi<i64>>::uninit();
        unsafe { tyrus_crypto_rand_random_int_result(2, 1, err.as_mut_ptr()) };
        let err = unsafe { err.assume_init() };
        assert_eq!(err.tag, 1);
        let payload = unsafe { &*err.payload.err };
        assert_eq!(unsafe { payload.code.as_str() }, "ERR_CRYPTO_INVALID_RANGE");
        assert_eq!(
            unsafe { payload.message.as_str() },
            "minimum must not be greater than maximum"
        );
    }

    #[test]
    fn aes_result_wrapper_returns_structured_invalid_key_error() {
        let mut result = std::mem::MaybeUninit::<CryptoResultFfi<TyrusString>>::uninit();
        unsafe {
            tyrus_crypto_cipher_aes_decrypt_str_result(
                ptr::null(),
                0,
                ptr::null(),
                0,
                ptr::null(),
                0,
                result.as_mut_ptr(),
            )
        };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe { &*result.payload.err };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_CRYPTO_INVALID_KEY_SIZE"
        );
        assert_eq!(
            unsafe { error.message.as_str() },
            "AES-256-GCM requires a 32-byte key"
        );
    }

    #[test]
    fn digest_and_hmac_result_wrappers_return_structured_input_errors() {
        let mut success = std::mem::MaybeUninit::<CryptoResultFfi<TyrusArray>>::uninit();
        unsafe {
            tyrus_crypto_digest_sha256_result(b"abc".as_ptr(), 3, success.as_mut_ptr());
        }
        let success = unsafe { success.assume_init() };
        assert_eq!(success.tag, 0);
        let digest_bytes = unsafe { &*success.payload.ok };
        assert_eq!(digest_bytes.length, 32);
        unsafe { free(digest_bytes.data.cast()) };

        let mut digest = std::mem::MaybeUninit::<CryptoResultFfi<TyrusArray>>::uninit();
        unsafe {
            tyrus_crypto_digest_sha256_result(ptr::null(), 1, digest.as_mut_ptr());
        }
        let digest = unsafe { digest.assume_init() };
        assert_eq!(digest.tag, 1);
        let digest_error = unsafe { &*digest.payload.err };
        assert_eq!(
            unsafe { digest_error.code.as_str() },
            "ERR_CRYPTO_INVALID_INPUT"
        );

        let mut hmac = std::mem::MaybeUninit::<CryptoResultFfi<TyrusArray>>::uninit();
        unsafe {
            tyrus_crypto_hmac_sha256_result(ptr::null(), 1, ptr::null(), 0, hmac.as_mut_ptr());
        }
        let hmac = unsafe { hmac.assume_init() };
        assert_eq!(hmac.tag, 1);
        let hmac_error = unsafe { &*hmac.payload.err };
        assert_eq!(
            unsafe { hmac_error.code.as_str() },
            "ERR_CRYPTO_INVALID_INPUT"
        );
    }

    #[test]
    fn jwt_sign_result_wrapper_returns_token_or_structured_error() {
        let payload = TyrusString::literal(r#"{"sub":"user-1"}"#);
        let key = TyrusString::literal("0123456789abcdef0123456789abcdef");
        let mut success = std::mem::MaybeUninit::<CryptoResultFfi<TyrusString>>::uninit();
        unsafe {
            tyrus_crypto_jwt_sign_result(payload, key, ptr::null(), success.as_mut_ptr());
        }
        let mut success = unsafe { success.assume_init() };
        assert_eq!(success.tag, 0);
        let token = unsafe { &*success.payload.ok };
        assert!(unsafe { token.as_str() }.contains('.'));
        let token = unsafe { std::mem::ManuallyDrop::take(&mut success.payload.ok) };
        unsafe { token.release() };

        let mut failure = std::mem::MaybeUninit::<CryptoResultFfi<TyrusString>>::uninit();
        unsafe {
            tyrus_crypto_jwt_sign_result(
                TyrusString::null(),
                TyrusString::literal("0123456789abcdef0123456789abcdef"),
                ptr::null(),
                failure.as_mut_ptr(),
            );
        }
        let failure = unsafe { failure.assume_init() };
        assert_eq!(failure.tag, 1);
        let error = unsafe { &*failure.payload.err };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_CRYPTO_JWT_INVALID_PAYLOAD"
        );
    }

    #[test]
    fn jwt_claim_result_wrappers_return_typed_success_and_structured_errors() {
        let key = TyrusString::literal("0123456789abcdef0123456789abcdef");
        let payload = TyrusString::literal(r#"{"sub":"user-1"}"#);
        let key_id = TyrusString::literal("key-1");
        let key_id_pointer = key_id.borrow();
        let sign_options = TyrusJwtSignOptions {
            algorithm: ptr::null(),
            audience: ptr::null(),
            expires_in: ptr::null(),
            issuer: ptr::null(),
            jwt_id: ptr::null(),
            key_id: &key_id_pointer,
            no_timestamp: ptr::null(),
            subject: ptr::null(),
        };
        let token = unsafe { tyrus_crypto_jwt_sign(payload, key, &sign_options) };
        assert!(!token.is_null());

        let mut verified = std::mem::MaybeUninit::<CryptoResultFfi<TyrusString>>::uninit();
        unsafe {
            tyrus_crypto_jwt_verify_result(
                token.borrow(),
                TyrusString::literal("0123456789abcdef0123456789abcdef"),
                ptr::null(),
                verified.as_mut_ptr(),
            );
        }
        let mut verified = unsafe { verified.assume_init() };
        assert_eq!(verified.tag, 0);
        let json = unsafe { std::mem::ManuallyDrop::take(&mut verified.payload.ok) };
        assert!(unsafe { json.as_str() }.contains("user-1"));
        unsafe { json.release() };

        let mut decoded = std::mem::MaybeUninit::<CryptoResultFfi<TyrusString>>::uninit();
        unsafe { tyrus_crypto_jwt_decode_unverified_result(token.borrow(), decoded.as_mut_ptr()) };
        let mut decoded = unsafe { decoded.assume_init() };
        assert_eq!(decoded.tag, 0);
        let json = unsafe { std::mem::ManuallyDrop::take(&mut decoded.payload.ok) };
        assert!(unsafe { json.as_str() }.contains("user-1"));
        unsafe { json.release() };

        let mut verify_error = std::mem::MaybeUninit::<CryptoResultFfi<TyrusString>>::uninit();
        unsafe {
            tyrus_crypto_jwt_verify_result(
                TyrusString::null(),
                TyrusString::literal("0123456789abcdef0123456789abcdef"),
                ptr::null(),
                verify_error.as_mut_ptr(),
            )
        };
        let verify_error = unsafe { verify_error.assume_init() };
        assert_eq!(verify_error.tag, 1);
        assert_eq!(
            unsafe { (*verify_error.payload.err).code.as_str() },
            "ERR_CRYPTO_JWT_MALFORMED"
        );

        let mut jwks_error = std::mem::MaybeUninit::<CryptoResultFfi<TyrusString>>::uninit();
        unsafe {
            tyrus_crypto_jwt_verify_with_jwks_result(
                token.borrow(),
                TyrusString::literal(r#"{"keys":[]}"#),
                ptr::null(),
                jwks_error.as_mut_ptr(),
            )
        };
        let jwks_error = unsafe { jwks_error.assume_init() };
        assert_eq!(jwks_error.tag, 1);
        assert_eq!(
            unsafe { (*jwks_error.payload.err).code.as_str() },
            "ERR_CRYPTO_JWKS_INVALID"
        );
        unsafe { token.release() };
    }

    #[test]
    fn crypto_error_result_writer_supports_success_payloads_larger_than_the_error() {
        type LargeResult = CryptoResultFfi<[u64; 8]>;
        assert_eq!(std::mem::offset_of!(LargeResult, payload), 8);
        let mut result = std::mem::MaybeUninit::<LargeResult>::zeroed();
        set_crypto_error(c"ERR_CRYPTO_JWT_INVALID_CLAIMS");
        unsafe { tyrus_crypto_take_error_result(result.as_mut_ptr().cast()) };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe { &*result.payload.err };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_CRYPTO_JWT_INVALID_CLAIMS"
        );
    }

    fn hex_bytes(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let text = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }

    unsafe fn take_array(array: TyrusArray) -> Vec<u8> {
        unsafe {
            let value = if array.length == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(array.data, array.length).to_vec()
            };
            free(array.data.cast());
            value
        }
    }

    unsafe fn take_string(value: TyrusString) -> String {
        let string = unsafe { value.as_str() }.to_owned();
        unsafe { value.release() };
        string
    }

    unsafe fn take_crypto_error() -> Option<String> {
        let error = tyrus_crypto_take_error();
        (!error.is_null()).then(|| unsafe { error.as_str() }.to_owned())
    }

    unsafe fn take_promise_crypto_string(
        promise: *mut promise::TyrusPromise,
    ) -> Result<String, String> {
        unsafe {
            assert_eq!(
                promise::wait_until_settled(promise),
                promise::TYRUS_PROMISE_FULFILLED
            );
            let slot =
                crate::tyrus_promise_take_value(promise).cast::<CryptoResultFfi<TyrusString>>();
            assert!(!slot.is_null());
            let result = if (*slot).tag == 0 {
                Ok(take_string(std::mem::replace(
                    &mut *(*slot).payload.ok,
                    TyrusString::null(),
                )))
            } else {
                Err((&(*slot).payload.err).code.as_str().to_owned())
            };
            drop_crypto_string_result(slot.cast());
            crate::tyrus_promise_free(promise);
            result
        }
    }

    unsafe fn take_promise_crypto_bool(
        promise: *mut promise::TyrusPromise,
    ) -> Result<bool, String> {
        unsafe {
            assert_eq!(
                promise::wait_until_settled(promise),
                promise::TYRUS_PROMISE_FULFILLED
            );
            let slot = crate::tyrus_promise_take_value(promise).cast::<CryptoResultFfi<u8>>();
            assert!(!slot.is_null());
            let result = if (*slot).tag == 0 {
                Ok(*(*slot).payload.ok != 0)
            } else {
                Err((&(*slot).payload.err).code.as_str().to_owned())
            };
            drop_argon2_verify_result(slot.cast());
            crate::tyrus_promise_free(promise);
            result
        }
    }

    #[test]
    fn sha256_matches_standard_vector_for_abc() {
        let mut output = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            tyrus_crypto_digest_sha256(b"abc".as_ptr(), 3, &mut output);
            assert_eq!(
                take_array(output),
                [
                    0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d,
                    0xae, 0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10,
                    0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
                ]
            );
        }
    }

    #[test]
    fn sha2_and_md5_match_published_abc_vectors() {
        unsafe fn digest_with(
            function: unsafe extern "C" fn(*const u8, usize, *mut TyrusArray),
        ) -> Vec<u8> {
            unsafe {
                let mut output = TyrusArray {
                    data: ptr::null_mut(),
                    length: 0,
                    capacity: 0,
                };
                function(b"abc".as_ptr(), 3, &mut output);
                take_array(output)
            }
        }

        unsafe {
            assert_eq!(
                digest_with(tyrus_crypto_digest_sha384),
                hex_bytes(
                    "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
                )
            );
            assert_eq!(
                digest_with(tyrus_crypto_digest_sha512),
                hex_bytes(
                    "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
                )
            );
            assert_eq!(
                digest_with(tyrus_crypto_digest_md5),
                hex_bytes("900150983cd24fb0d6963f7d28e17f72")
            );
        }
    }

    #[test]
    fn sha256_distinguishes_empty_input_from_embedded_zero_bytes() {
        unsafe fn digest(data: &[u8]) -> Vec<u8> {
            unsafe {
                let mut output = TyrusArray {
                    data: ptr::null_mut(),
                    length: 0,
                    capacity: 0,
                };
                tyrus_crypto_digest_sha256(data.as_ptr(), data.len(), &mut output);
                take_array(output)
            }
        }

        unsafe {
            assert_eq!(
                digest(&[]),
                hex_bytes("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
            );
            assert_eq!(
                digest(&[0]),
                hex_bytes("6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d")
            );
            assert_ne!(digest(&[0, 1, 0]), digest(&[0, 1]));
        }
    }

    #[test]
    fn hmac_sha256_matches_rfc_4231_case_one() {
        let key = [0x0b; 20];
        let mut output = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            tyrus_crypto_hmac_sha256(
                b"Hi There".as_ptr(),
                8,
                key.as_ptr(),
                key.len(),
                &mut output,
            );
            assert_eq!(
                take_array(output),
                [
                    0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf,
                    0x0b, 0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7, 0x26, 0xe9,
                    0x37, 0x6c, 0x2e, 0x32, 0xcf, 0xf7,
                ]
            );
        }
    }

    #[test]
    fn hmac_sha384_and_sha512_match_rfc_4231_case_one() {
        unsafe fn hmac_with(
            function: unsafe extern "C" fn(*const u8, usize, *const u8, usize, *mut TyrusArray),
        ) -> Vec<u8> {
            unsafe {
                let key = [0x0b; 20];
                let mut output = TyrusArray {
                    data: ptr::null_mut(),
                    length: 0,
                    capacity: 0,
                };
                function(
                    b"Hi There".as_ptr(),
                    8,
                    key.as_ptr(),
                    key.len(),
                    &mut output,
                );
                take_array(output)
            }
        }

        unsafe {
            assert_eq!(
                hmac_with(tyrus_crypto_hmac_sha384),
                hex_bytes(
                    "afd03944d84895626b0825f4ab46907f15f9dadbe4101ec682aa034c7cebc59cfaea9ea9076ede7f4af152e8b2fa9cb6"
                )
            );
            assert_eq!(
                hmac_with(tyrus_crypto_hmac_sha512),
                hex_bytes(
                    "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
                )
            );
        }
    }

    #[test]
    fn timing_safe_equal_handles_empty_equal_and_different_inputs() {
        unsafe {
            assert!(tyrus_crypto_timing_safe_equal(
                ptr::null(),
                0,
                ptr::null(),
                0
            ));
            assert!(tyrus_crypto_timing_safe_equal(
                b"same".as_ptr(),
                4,
                b"same".as_ptr(),
                4
            ));
            assert!(!tyrus_crypto_timing_safe_equal(
                b"same".as_ptr(),
                4,
                b"diff".as_ptr(),
                4
            ));
            assert!(!tyrus_crypto_timing_safe_equal(
                b"a".as_ptr(),
                1,
                b"aa".as_ptr(),
                2
            ));
        }
    }

    #[test]
    fn argon2_rehash_check_validates_phc_and_compares_the_complete_profile() {
        const CURRENT: &str = "$argon2id$v=19$m=65536,t=3,p=4$c29tZXNhbHQxMjM0NTY3OA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        const WEAKER: &str = "$argon2id$v=19$m=32768,t=3,p=4$c29tZXNhbHQxMjM0NTY3OA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        const WRONG_ALGORITHM: &str = "$argon2i$v=19$m=65536,t=3,p=4$c29tZXNhbHQxMjM0NTY3OA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        const SHORT_SALT: &str =
            "$argon2id$v=19$m=65536,t=3,p=4$c2hvcnQ$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

        assert_eq!(argon2_hash_needs_rehash(CURRENT), Ok(false));
        assert_eq!(argon2_hash_needs_rehash(WEAKER), Ok(true));
        assert_eq!(argon2_hash_needs_rehash(WRONG_ALGORITHM), Ok(true));
        assert_eq!(argon2_hash_needs_rehash(SHORT_SALT), Ok(true));
        assert!(argon2_hash_needs_rehash("not-a-phc-string").is_err());
        assert!(argon2_hash_needs_rehash(&"x".repeat(MAX_PHC_BYTES + 1)).is_err());

        unsafe {
            let mut valid = std::mem::MaybeUninit::<CryptoResultFfi<u8>>::uninit();
            tyrus_crypto_argon2_needs_rehash_result(
                TyrusString::literal(CURRENT),
                valid.as_mut_ptr(),
            );
            let valid = valid.assume_init();
            assert_eq!(valid.tag, 0);
            assert_eq!(*valid.payload.ok, 0);

            let mut invalid = std::mem::MaybeUninit::<CryptoResultFfi<u8>>::uninit();
            tyrus_crypto_argon2_needs_rehash_result(
                TyrusString::literal("not-a-phc-string"),
                invalid.as_mut_ptr(),
            );
            let invalid = invalid.assume_init();
            assert_eq!(invalid.tag, 1);
            assert_eq!(
                invalid.payload.err.code.as_str(),
                "ERR_CRYPTO_ARGON2_INVALID_HASH"
            );
        }
    }

    #[test]
    fn argon2_supports_binary_passwords_and_requires_the_matching_pepper() {
        let password = b"binary\0password";
        let pepper = b"application-pepper";
        let hash = hash_argon2id_with_salt(
            password,
            Some(pepper),
            b"0123456789abcdef",
            Argon2Profile::default(),
        )
        .expect("peppered hash must be created");
        assert!(hash.contains("data="));
        assert_eq!(argon2_hash_needs_rehash(&hash), Ok(false));
        assert_eq!(
            verify_argon2_password(&hash, password, Some(pepper)),
            Ok(true)
        );
        assert_eq!(
            verify_argon2_password(&hash, b"wrong", Some(pepper)),
            Ok(false)
        );
        assert_eq!(
            verify_argon2_password(&hash, password, None),
            Err("ERR_CRYPTO_ARGON2_PEPPER_REQUIRED")
        );
        assert_eq!(
            verify_argon2_password(&hash, password, Some(b"wrong-pepper")),
            Ok(false)
        );
        let unicode = "contraseña-🔐".as_bytes();
        let unicode_hash =
            hash_argon2id_with_salt(unicode, None, b"fedcba9876543210", Argon2Profile::default())
                .unwrap();
        assert_eq!(
            verify_argon2_password(&unicode_hash, unicode, None),
            Ok(true)
        );
    }

    #[test]
    fn argon2_ffi_hashes_verifies_and_rejects_invalid_options() {
        let password = b"ffi-password";
        let pepper = b"ffi-application-pepper";
        let secret = TyrusArray {
            data: pepper.as_ptr().cast_mut(),
            length: pepper.len(),
            capacity: pepper.len(),
        };
        let memory_cost = 19_456;
        let time_cost = 2;
        let parallelism = 1;
        let algorithm = TyrusString::literal("argon2id");
        let algorithm_pointer = algorithm.borrow();
        let hash_options = TyrusArgon2HashOptions {
            memory_cost: &memory_cost,
            parallelism: &parallelism,
            secret: &secret,
            time_cost: &time_cost,
            algorithm: &algorithm_pointer,
        };

        unsafe {
            let hash = take_promise_crypto_string(tyrus_crypto_argon2_hash(
                password.as_ptr(),
                password.len(),
                &hash_options,
            ))
            .unwrap();
            assert!(hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1,"));
            assert!(hash.contains("data="));

            // SAFETY: `hash` outlives every call it is lent to below.
            let hash_c = TyrusString::lend(&hash).expect("within the limit");
            let verify_options = TyrusArgon2VerifyOptions { secret: &secret };
            assert_eq!(
                take_promise_crypto_bool(tyrus_crypto_argon2_verify(
                    hash_c.borrow(),
                    password.as_ptr(),
                    password.len(),
                    &verify_options,
                )),
                Ok(true)
            );
            assert_eq!(
                take_promise_crypto_bool(tyrus_crypto_argon2_verify(
                    hash_c.borrow(),
                    b"wrong-password".as_ptr(),
                    b"wrong-password".len(),
                    &verify_options,
                )),
                Ok(false)
            );
            assert!(tyrus_crypto_argon2_needs_rehash(hash_c.borrow()));

            let empty_secret = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            let invalid_verify_options = TyrusArgon2VerifyOptions {
                secret: &empty_secret,
            };
            assert_eq!(
                take_promise_crypto_bool(tyrus_crypto_argon2_verify(
                    hash_c.borrow(),
                    password.as_ptr(),
                    password.len(),
                    &invalid_verify_options,
                )),
                Err("ERR_CRYPTO_ARGON2_INVALID_PEPPER".to_string())
            );

            let invalid_memory_cost = 1;
            let invalid_hash_options = TyrusArgon2HashOptions {
                memory_cost: &invalid_memory_cost,
                parallelism: ptr::null(),
                secret: ptr::null(),
                time_cost: ptr::null(),
                algorithm: ptr::null(),
            };
            assert_eq!(
                take_promise_crypto_string(tyrus_crypto_argon2_hash(
                    password.as_ptr(),
                    password.len(),
                    &invalid_hash_options,
                )),
                Err("ERR_CRYPTO_ARGON2_PARAMETERS".to_string())
            );

            let invalid_hash = TyrusString::literal("not-a-phc-string");
            assert!(!tyrus_crypto_argon2_needs_rehash(invalid_hash.borrow()));
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_ARGON2_INVALID_HASH")
            );
        }
    }

    #[test]
    fn public_ffi_rejects_invalid_pointers_lengths_and_ranges() {
        unsafe {
            tyrus_crypto_clear_error();
            assert!(!tyrus_crypto_timing_safe_equal(
                ptr::null(),
                1,
                ptr::null(),
                1
            ));
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_INPUT")
            );

            tyrus_crypto_digest_sha256(b"data".as_ptr(), 4, ptr::null_mut());
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_OUTPUT")
            );

            let mut output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            tyrus_crypto_rand_random_bytes((MAX_RANDOM_BYTES + 1) as u32, &mut output);
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_RANDOM_LIMIT")
            );
            assert_eq!(tyrus_crypto_rand_random_int(2, 1), 0);
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_RANGE")
            );
        }
    }

    #[test]
    fn public_ffi_rejects_every_deterministic_invalid_input_class() {
        unsafe {
            let mut output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };

            for hmac in [
                tyrus_crypto_hmac_sha256
                    as unsafe extern "C" fn(*const u8, usize, *const u8, usize, *mut TyrusArray),
                tyrus_crypto_hmac_sha384,
                tyrus_crypto_hmac_sha512,
            ] {
                hmac(ptr::null(), 1, b"key".as_ptr(), 3, &mut output);
                assert_eq!(
                    take_crypto_error().as_deref(),
                    Some("ERR_CRYPTO_INVALID_INPUT")
                );
            }
            for digest in [
                tyrus_crypto_digest_sha256
                    as unsafe extern "C" fn(*const u8, usize, *mut TyrusArray),
                tyrus_crypto_digest_sha384,
                tyrus_crypto_digest_sha512,
                tyrus_crypto_digest_md5,
            ] {
                digest(ptr::null(), 1, &mut output);
                assert_eq!(
                    take_crypto_error().as_deref(),
                    Some("ERR_CRYPTO_INVALID_INPUT")
                );
            }

            let key = [0u8; AES_KEY_LENGTH];
            let short_key = [0u8; AES_KEY_LENGTH - 1];
            tyrus_crypto_cipher_aes_encrypt(
                ptr::null(),
                1,
                key.as_ptr(),
                key.len(),
                ptr::null(),
                0,
                &mut output,
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_INPUT")
            );
            tyrus_crypto_cipher_aes_encrypt(
                b"data".as_ptr(),
                4,
                short_key.as_ptr(),
                short_key.len(),
                ptr::null(),
                0,
                &mut output,
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_KEY_SIZE")
            );
            assert!(
                tyrus_crypto_cipher_aes_decrypt_str(
                    ptr::null(),
                    1,
                    key.as_ptr(),
                    key.len(),
                    ptr::null(),
                    0,
                )
                .is_null()
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_INPUT")
            );
            tyrus_crypto_cipher_aes_decrypt_bytes(
                b"invalid".as_ptr(),
                7,
                key.as_ptr(),
                key.len(),
                ptr::null(),
                0,
                &mut output,
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_ENVELOPE")
            );

            assert_eq!(
                take_promise_crypto_string(tyrus_crypto_argon2_hash(ptr::null(), 1, ptr::null())),
                Err("ERR_CRYPTO_ARGON2_INVALID_PASSWORD".to_string())
            );
            assert_eq!(
                take_promise_crypto_bool(tyrus_crypto_argon2_verify(
                    TyrusString::null(),
                    b"password".as_ptr(),
                    8,
                    ptr::null(),
                )),
                Err("ERR_CRYPTO_ARGON2_INVALID_HASH".to_string())
            );
            let hash = TyrusString::literal("$argon2id$v=19$m=65536,t=3,p=4$bad$bad");
            assert_eq!(
                take_promise_crypto_bool(tyrus_crypto_argon2_verify(
                    hash.borrow(),
                    ptr::null(),
                    1,
                    ptr::null(),
                )),
                Err("ERR_CRYPTO_ARGON2_INVALID_PASSWORD".to_string())
            );
            assert!(!tyrus_crypto_argon2_needs_rehash(TyrusString::null()));
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_ARGON2_INVALID_HASH")
            );

            let oversized_claims_text = format!(r#"{{"x":"{}"}}"#, "a".repeat(MAX_JWT_BYTES));
            // SAFETY: the text outlives every call it is lent to below.
            let oversized_claims =
                TyrusString::lend(&oversized_claims_text).expect("within the limit");
            let secret = TyrusString::literal("0123456789abcdef0123456789abcdef");
            assert!(
                tyrus_crypto_jwt_sign(oversized_claims.borrow(), secret.borrow(), ptr::null(),)
                    .is_null()
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_JWT_INPUT_TOO_LARGE")
            );
            let oversized_token_text = "a".repeat(MAX_JWT_BYTES + 1);
            // SAFETY: the text outlives every call it is lent to below.
            let oversized_token =
                TyrusString::lend(&oversized_token_text).expect("within the limit");
            assert!(
                tyrus_crypto_jwt_verify(oversized_token.borrow(), secret.borrow(), ptr::null(),)
                    .is_null()
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_JWT_INPUT_TOO_LARGE")
            );
            assert!(tyrus_crypto_jwt_decode_unverified(oversized_token.borrow()).is_null());
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_JWT_INPUT_TOO_LARGE")
            );

            assert!(tyrus_crypto_digest_create(TyrusString::null()).is_null());
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_DIGEST_ALGORITHM")
            );
            let algorithm = TyrusString::literal("sha256");
            let stream = tyrus_crypto_digest_create(algorithm.borrow());
            tyrus_crypto_digest_stream_update(stream, ptr::null(), 1);
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_INPUT")
            );
            tyrus_crypto_digest_stream_free(stream);
        }
    }

    #[test]
    fn argon2id_matches_rfc_9106_version_19_vector() {
        use argon2::{Algorithm, Argon2, AssociatedData, ParamsBuilder, Version};

        let params = ParamsBuilder::new()
            .m_cost(32)
            .t_cost(3)
            .p_cost(4)
            .data(AssociatedData::new(&[0x04; 12]).unwrap())
            .build()
            .unwrap();
        let argon2 =
            Argon2::new_with_secret(&[0x03; 8], Algorithm::Argon2id, Version::V0x13, params)
                .unwrap();
        let mut output = [0u8; 32];
        argon2
            .hash_password_into(&[0x01; 32], &[0x02; 16], &mut output)
            .unwrap();
        assert_eq!(
            output.as_slice(),
            hex_bytes("0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659")
        );
    }

    #[test]
    fn argon2_rejects_oversized_input_before_reading_it() {
        let byte = 7u8;
        assert!(
            unsafe { argon2_input(&byte, MAX_PASSWORD_BYTES + 1, MAX_PASSWORD_BYTES) }.is_err()
        );
        assert!(unsafe { argon2_input(&byte, MAX_PEPPER_BYTES + 1, MAX_PEPPER_BYTES) }.is_err());
        assert!(unsafe { argon2_input(std::ptr::null(), 1, MAX_PASSWORD_BYTES) }.is_err());
    }

    #[test]
    fn argon2_concurrency_limit_rejects_instead_of_queueing() {
        let slots = Arc::new(tokio::sync::Semaphore::new(4));
        let permits = (0..4)
            .map(|_| slots.clone().try_acquire_owned().unwrap())
            .collect::<Vec<_>>();
        assert!(slots.clone().try_acquire_owned().is_err());
        drop(permits);
        assert!(slots.try_acquire_owned().is_ok());
    }

    #[test]
    fn aes_envelope_authenticates_header_payload_and_aad() {
        let key = [7u8; AES_KEY_LENGTH];
        let aad = b"record:42";
        let plaintext = b"sensitive binary\0payload";
        let mut encrypted = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            tyrus_crypto_cipher_aes_encrypt(
                plaintext.as_ptr(),
                plaintext.len(),
                key.as_ptr(),
                key.len(),
                aad.as_ptr(),
                aad.len(),
                &mut encrypted,
            );
            assert!(
                std::slice::from_raw_parts(encrypted.data, encrypted.length)
                    .starts_with(AES_ENVELOPE_HEADER)
            );
            let mut decrypted = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            tyrus_crypto_cipher_aes_decrypt_bytes(
                encrypted.data,
                encrypted.length,
                key.as_ptr(),
                key.len(),
                aad.as_ptr(),
                aad.len(),
                &mut decrypted,
            );
            assert_eq!(take_array(decrypted), plaintext);
            free(encrypted.data.cast());
        }
    }

    #[test]
    fn aes_rejects_tampering_wrong_aad_wrong_key_and_truncation() {
        let key = [7u8; AES_KEY_LENGTH];
        let wrong_key = [8u8; AES_KEY_LENGTH];
        let aad = b"record:42";
        let mut encrypted = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            tyrus_crypto_cipher_aes_encrypt(
                b"authenticated".as_ptr(),
                13,
                key.as_ptr(),
                key.len(),
                aad.as_ptr(),
                aad.len(),
                &mut encrypted,
            );
            let mut envelope = take_array(encrypted);
            assert!(decrypt_aes_bytes(&envelope, &key, b"wrong-aad").is_err());
            assert!(decrypt_aes_bytes(&envelope, &wrong_key, aad).is_err());
            let last = envelope.len() - 1;
            envelope[last] ^= 1;
            assert!(decrypt_aes_bytes(&envelope, &key, aad).is_err());
            envelope[last] ^= 1;
            let mut truncated_ciphertext = envelope.clone();
            truncated_ciphertext.remove(AES_ENVELOPE_HEADER.len() + AES_NONCE_LENGTH);
            assert!(decrypt_aes_bytes(&truncated_ciphertext, &key, aad).is_err());
            assert!(decrypt_aes_bytes(&envelope[..envelope.len() - 1], &key, aad).is_err());
            let mut truncated_nonce = Vec::from(AES_ENVELOPE_HEADER.as_slice());
            truncated_nonce.extend_from_slice(
                &envelope[AES_ENVELOPE_HEADER.len()..][..AES_NONCE_LENGTH - 1 + AES_TAG_LENGTH],
            );
            assert!(decrypt_aes_bytes(&truncated_nonce, &key, aad).is_err());
            assert!(decrypt_aes_bytes(&envelope[..AES_ENVELOPE_HEADER.len()], &key, aad).is_err());
        }
    }

    #[test]
    fn aes_random_nonces_do_not_repeat_in_a_large_process_local_batch() {
        let key = [9u8; AES_KEY_LENGTH];
        let mut nonces = HashSet::new();
        for _ in 0..1024 {
            let mut encrypted = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            unsafe {
                tyrus_crypto_cipher_aes_encrypt(
                    b"payload".as_ptr(),
                    7,
                    key.as_ptr(),
                    key.len(),
                    ptr::null(),
                    0,
                    &mut encrypted,
                );
                let envelope = take_array(encrypted);
                let nonce_start = AES_ENVELOPE_HEADER.len();
                assert!(
                    nonces.insert(envelope[nonce_start..nonce_start + AES_NONCE_LENGTH].to_vec())
                );
            }
        }
    }

    #[test]
    fn aes_text_decryption_rejects_authenticated_non_utf8_plaintext() {
        let key = [11u8; AES_KEY_LENGTH];
        let mut encrypted = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            tyrus_crypto_clear_error();
            tyrus_crypto_cipher_aes_encrypt(
                [0xff, 0xfe].as_ptr(),
                2,
                key.as_ptr(),
                key.len(),
                ptr::null(),
                0,
                &mut encrypted,
            );
            let text = tyrus_crypto_cipher_aes_decrypt_str(
                encrypted.data,
                encrypted.length,
                key.as_ptr(),
                key.len(),
                ptr::null(),
                0,
            );
            assert!(text.is_null());
            let error = tyrus_crypto_take_error();
            assert_eq!(error.as_str(), "ERR_CRYPTO_INVALID_UTF8");
            free(encrypted.data.cast());
        }
    }

    #[test]
    fn aes256_gcm_matches_nist_empty_plaintext_vector() {
        use aes_gcm::{
            Aes256Gcm, KeyInit,
            aead::{Aead, Nonce, Payload},
        };

        let cipher = Aes256Gcm::new_from_slice(&[0u8; AES_KEY_LENGTH]).unwrap();
        let nonce = Nonce::<Aes256Gcm>::from([0u8; AES_NONCE_LENGTH]);
        let ciphertext = cipher
            .encrypt(&nonce, Payload { msg: &[], aad: &[] })
            .unwrap();
        assert_eq!(ciphertext, hex_bytes("530f8afbc74536b9a963b4f1c4cb738b"));
    }

    #[test]
    fn aes256_gcm_passes_every_supported_project_wycheproof_vector() {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/wycheproof/aes_gcm_test.json")).unwrap();
        let mut executed = 0usize;
        let mut valid = 0usize;
        let mut invalid = 0usize;
        for group in corpus["testGroups"].as_array().unwrap() {
            if group["keySize"] != 256 || group["ivSize"] != 96 || group["tagSize"] != 128 {
                continue;
            }
            for test in group["tests"].as_array().unwrap() {
                let test_id = test["tcId"].as_u64().unwrap();
                let key = hex_bytes(test["key"].as_str().unwrap());
                let nonce: [u8; AES_NONCE_LENGTH] =
                    hex_bytes(test["iv"].as_str().unwrap()).try_into().unwrap();
                let aad = hex_bytes(test["aad"].as_str().unwrap());
                let plaintext = hex_bytes(test["msg"].as_str().unwrap());
                let mut ciphertext_and_tag = hex_bytes(test["ct"].as_str().unwrap());
                ciphertext_and_tag.extend(hex_bytes(test["tag"].as_str().unwrap()));
                match test["result"].as_str().unwrap() {
                    "valid" => {
                        assert_eq!(
                            encrypt_aes_gcm_raw(&plaintext, &key, &nonce, &aad).unwrap(),
                            ciphertext_and_tag,
                            "Wycheproof tcId {test_id} encryption mismatch"
                        );
                        assert_eq!(
                            decrypt_aes_gcm_raw(&ciphertext_and_tag, &key, &nonce, &aad)
                                .unwrap()
                                .as_slice(),
                            plaintext,
                            "Wycheproof tcId {test_id} decryption mismatch"
                        );
                        valid += 1;
                    }
                    "invalid" => {
                        assert!(
                            decrypt_aes_gcm_raw(&ciphertext_and_tag, &key, &nonce, &aad).is_err(),
                            "Wycheproof tcId {test_id} accepted invalid ciphertext"
                        );
                        invalid += 1;
                    }
                    result => panic!("unexpected Wycheproof result `{result}`"),
                }
                executed += 1;
            }
        }
        assert_eq!((executed, valid, invalid), (66, 39, 27));
    }

    #[test]
    fn injected_entropy_failure_never_exposes_a_partially_filled_buffer() {
        let mut destination = [0u8; AES_NONCE_LENGTH];
        let result = fill_random_using(&mut destination, |bytes| {
            bytes[..4].copy_from_slice(&[1, 2, 3, 4]);
            Err(())
        });
        assert_eq!(result, Err(()));
        assert_eq!(destination, [0u8; AES_NONCE_LENGTH]);
    }

    #[test]
    fn zeroizing_guard_clears_runtime_controlled_memory_on_drop() {
        struct BorrowedSecret<'a>(&'a mut [u8]);

        impl Zeroize for BorrowedSecret<'_> {
            fn zeroize(&mut self) {
                self.0.zeroize();
            }
        }

        let mut key_copy = [0x5au8; AES_KEY_LENGTH];
        {
            let guarded = Zeroizing::new(BorrowedSecret(key_copy.as_mut_slice()));
            assert!(guarded.0.iter().all(|byte| *byte == 0x5a));
        }
        assert_eq!(key_copy, [0u8; AES_KEY_LENGTH]);

        let mut password_copy = vec![0x7bu8; 128];
        {
            let guarded = Zeroizing::new(BorrowedSecret(password_copy.as_mut_slice()));
            assert!(guarded.0.iter().all(|byte| *byte == 0x7b));
        }
        assert!(password_copy.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn digest_stream_can_digest_reset_and_be_disposed() {
        unsafe {
            for (algorithm, digest_length) in [("sha256", 32), ("sha384", 48), ("sha512", 64)] {
                let algorithm = crate::strings::alloc_string(algorithm);
                let stream = tyrus_crypto_digest_create(algorithm.borrow());
                tyrus_crypto_digest_stream_update(stream, b"abc".as_ptr(), 3);
                let mut first = TyrusArray {
                    data: ptr::null_mut(),
                    length: 0,
                    capacity: 0,
                };
                tyrus_crypto_digest_stream_digest(stream, &mut first);
                assert_eq!(take_array(first).len(), digest_length);
                tyrus_crypto_digest_stream_reset(stream);
                let mut empty = TyrusArray {
                    data: ptr::null_mut(),
                    length: 0,
                    capacity: 0,
                };
                tyrus_crypto_digest_stream_digest(stream, &mut empty);
                assert_eq!(take_array(empty).len(), digest_length);
                tyrus_crypto_digest_stream_free(stream);
                algorithm.release();
            }

            let invalid = TyrusString::literal("sha1");
            assert!(tyrus_crypto_digest_create(invalid.borrow()).is_null());
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_DIGEST_ALGORITHM")
            );
            tyrus_crypto_digest_stream_update(ptr::null_mut(), b"abc".as_ptr(), 3);
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_STREAM")
            );
            let mut output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            tyrus_crypto_digest_stream_digest(ptr::null_mut(), &mut output);
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_STREAM")
            );
            tyrus_crypto_digest_stream_reset(ptr::null_mut());
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_INVALID_STREAM")
            );
        }
    }

    #[test]
    fn jwt_sign_verify_and_unverified_decode_are_explicit() {
        let payload = TyrusString::literal(r#"{"sub":"user-42"}"#);
        let secret = TyrusString::literal("0123456789abcdef0123456789abcdef");
        unsafe {
            let token = tyrus_crypto_jwt_sign(payload.borrow(), secret.borrow(), ptr::null());
            let verified = tyrus_crypto_jwt_verify(token.borrow(), secret.borrow(), ptr::null());
            let decoded = tyrus_crypto_jwt_decode_unverified(token.borrow());
            assert!(take_string(verified).contains(r#""sub":"user-42""#));
            assert!(take_string(decoded).contains(r#""sub":"user-42""#));
            token.release();
        }
    }

    #[test]
    fn jwt_ffi_options_round_trip_preserves_claims() {
        let payload = TyrusString::literal(r#"{"role":"admin"}"#);
        let secret = TyrusString::literal("0123456789abcdef0123456789abcdef");
        let algorithm = TyrusString::literal("HS384");
        let audience = TyrusString::literal("tyrus-api");
        let expires_in = TyrusString::literal("5m");
        let issuer = TyrusString::literal("https://issuer.example");
        let jwt_id = TyrusString::literal("token-42");
        let key_id = TyrusString::literal("active-key");
        let subject = TyrusString::literal("user-42");
        let algorithm_pointer = algorithm.borrow();
        let audience_pointer = audience.borrow();
        let expires_in_pointer = expires_in.borrow();
        let issuer_pointer = issuer.borrow();
        let jwt_id_pointer = jwt_id.borrow();
        let key_id_pointer = key_id.borrow();
        let subject_pointer = subject.borrow();
        let no_timestamp = false;
        let sign_options = TyrusJwtSignOptions {
            algorithm: &algorithm_pointer,
            audience: &audience_pointer,
            expires_in: &expires_in_pointer,
            issuer: &issuer_pointer,
            jwt_id: &jwt_id_pointer,
            key_id: &key_id_pointer,
            no_timestamp: &no_timestamp,
            subject: &subject_pointer,
        };
        let algorithm_pointers = [algorithm.borrow()];
        let algorithms = TyrusArray {
            data: algorithm_pointers.as_ptr().cast_mut().cast::<u8>(),
            length: algorithm_pointers.len(),
            capacity: algorithm_pointers.len(),
        };
        let verify_options = TyrusJwtVerifyOptions {
            algorithms: &algorithms,
            audience: &audience_pointer,
            clock_tolerance: ptr::null(),
            issuer: &issuer_pointer,
            max_age: ptr::null(),
            subject: &subject_pointer,
        };
        unsafe {
            let token = tyrus_crypto_jwt_sign(payload.borrow(), secret.borrow(), &sign_options);
            assert!(
                !token.is_null(),
                "JWT signing failed: {:?}",
                take_crypto_error()
            );

            let verified =
                tyrus_crypto_jwt_verify(token.borrow(), secret.borrow(), &verify_options);
            assert!(
                !verified.is_null(),
                "JWT verification failed: {:?}",
                take_crypto_error()
            );
            let verified = take_string(verified);
            assert!(verified.contains(r#""role":"admin""#));
            assert!(verified.contains(r#""aud":"tyrus-api""#));
            assert!(verified.contains(r#""iss":"https://issuer.example""#));
            assert!(verified.contains(r#""jti":"token-42""#));
            assert!(verified.contains(r#""sub":"user-42""#));

            let decoded = tyrus_crypto_jwt_decode_unverified(token.borrow());
            assert!(
                !decoded.is_null(),
                "JWT decoding failed: {:?}",
                take_crypto_error()
            );
            assert!(take_string(decoded).contains(r#""jti":"token-42""#));
            token.release();
        }
    }

    #[test]
    fn jwt_ffi_rejects_invalid_option_layouts_and_limits() {
        let invalid_algorithms = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let excessive_tolerance = 24 * 60 * 60 + 1;
        let invalid_options = TyrusJwtVerifyOptions {
            algorithms: &invalid_algorithms,
            audience: ptr::null(),
            clock_tolerance: &excessive_tolerance,
            issuer: ptr::null(),
            max_age: ptr::null(),
            subject: ptr::null(),
        };
        unsafe {
            assert_eq!(
                read_jwt_verify_options(&invalid_options)
                    .err()
                    .unwrap()
                    .to_bytes(),
                b"ERR_CRYPTO_JWT_INVALID_OPTION"
            );
        }
        assert_eq!(parse_jwt_duration("2h"), Ok(2 * 60 * 60));
        assert_eq!(parse_jwt_duration("3m"), Ok(3 * 60));
        assert_eq!(parse_jwt_duration("30"), Ok(30));
    }

    #[test]
    fn jwt_public_ffi_rejects_invalid_inputs_without_network_access() {
        let payload = TyrusString::literal("{}");
        let secret = TyrusString::literal("0123456789abcdef0123456789abcdef");
        let invalid_algorithm = TyrusString::literal("none");
        let invalid_algorithm_pointer = invalid_algorithm.borrow();
        let invalid_sign_options = TyrusJwtSignOptions {
            algorithm: &invalid_algorithm_pointer,
            audience: ptr::null(),
            expires_in: ptr::null(),
            issuer: ptr::null(),
            jwt_id: ptr::null(),
            key_id: ptr::null(),
            no_timestamp: ptr::null(),
            subject: ptr::null(),
        };

        unsafe {
            tyrus_crypto_set_invalid_jwt_claims_error();
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_JWT_INVALID_CLAIMS")
            );

            assert!(
                tyrus_crypto_jwt_sign(TyrusString::null(), secret.borrow(), ptr::null()).is_null()
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_JWT_INVALID_PAYLOAD")
            );
            assert!(
                tyrus_crypto_jwt_sign(payload.borrow(), TyrusString::null(), ptr::null()).is_null()
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_JWT_INVALID_KEY")
            );
            assert!(
                tyrus_crypto_jwt_sign(payload.borrow(), secret.borrow(), &invalid_sign_options,)
                    .is_null()
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_JWT_INVALID_ALGORITHM")
            );

            assert!(
                tyrus_crypto_jwt_verify(TyrusString::null(), secret.borrow(), ptr::null())
                    .is_null()
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_JWT_MALFORMED")
            );
            assert!(
                tyrus_crypto_jwt_verify_with_jwks(
                    TyrusString::null(),
                    TyrusString::null(),
                    ptr::null()
                )
                .is_null()
            );
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_JWT_MALFORMED")
            );
            assert!(tyrus_crypto_jwt_decode_unverified(TyrusString::null()).is_null());
            assert_eq!(
                take_crypto_error().as_deref(),
                Some("ERR_CRYPTO_JWT_MALFORMED")
            );

            assert_eq!(
                take_promise_crypto_string(tyrus_crypto_jwt_fetch_jwks(
                    TyrusString::null(),
                    ptr::null(),
                )),
                Err("ERR_CRYPTO_JWKS_INVALID_URL".to_string())
            );
            let oversized_url_text = "x".repeat(2049);
            // SAFETY: the text outlives the call it is lent to.
            let oversized_url = TyrusString::lend(&oversized_url_text).expect("within the limit");
            assert_eq!(
                take_promise_crypto_string(tyrus_crypto_jwt_fetch_jwks(
                    oversized_url.borrow(),
                    ptr::null(),
                )),
                Err("ERR_CRYPTO_JWKS_INVALID_URL".to_string())
            );
            let insecure_url = TyrusString::literal("http://example.com/jwks");
            let force_refresh = true;
            let fetch_options = TyrusJwksFetchOptions {
                force_refresh: &force_refresh,
            };
            assert_eq!(
                take_promise_crypto_string(tyrus_crypto_jwt_fetch_jwks(
                    insecure_url.borrow(),
                    &fetch_options,
                )),
                Err("ERR_CRYPTO_JWKS_INVALID_URL".to_string())
            );
        }
    }

    #[test]
    fn jwt_rejects_weak_keys_malformed_inputs_wrong_signatures_and_expired_tokens() {
        let secret = b"0123456789abcdef0123456789abcdef";
        let wrong_secret = b"fedcba9876543210fedcba9876543210";

        assert_eq!(
            sign_jwt("{}", b"short", 1, &JwtSignOptions::default()).unwrap_err(),
            c"ERR_CRYPTO_JWT_WEAK_KEY"
        );
        assert_eq!(
            sign_jwt("[]", secret, 1, &JwtSignOptions::default()).unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_PAYLOAD"
        );
        assert_eq!(
            decode_jwt_unverified("not-a-token").unwrap_err(),
            c"ERR_CRYPTO_JWT_MALFORMED"
        );

        let token = sign_jwt(
            r#"{"sub":"user-42"}"#,
            secret,
            123,
            &JwtSignOptions::default(),
        )
        .unwrap();
        assert_eq!(
            verify_jwt(&token, wrong_secret, &JwtVerifyOptions::default()).unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_SIGNATURE"
        );
        assert!(
            verify_jwt(&token, secret, &JwtVerifyOptions::default())
                .unwrap()
                .contains(r#""iat":123"#)
        );

        let expired = sign_jwt(r#"{"exp":1}"#, secret, 1, &JwtSignOptions::default()).unwrap();
        assert_eq!(
            verify_jwt(&expired, secret, &JwtVerifyOptions::default()).unwrap_err(),
            c"ERR_CRYPTO_JWT_EXPIRED"
        );
    }

    #[test]
    fn jwt_hs256_verification_rejects_algorithm_confusion() {
        let secret = b"0123456789abcdef0123456789abcdef";
        assert!(matches!(
            verify_jwt(
                "eyJhbGciOiJub25lIn0.e30.",
                secret,
                &JwtVerifyOptions::default(),
            )
            .unwrap_err(),
            error if error == c"ERR_CRYPTO_JWT_INVALID_ALGORITHM" || error == c"ERR_CRYPTO_JWT_MALFORMED"
        ));
        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS384),
            &serde_json::json!({"sub": "user-42"}),
            &jsonwebtoken::EncodingKey::from_secret(secret),
        )
        .unwrap();
        assert_eq!(
            verify_jwt(&token, secret, &JwtVerifyOptions::default()).unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_ALGORITHM"
        );
    }

    #[test]
    fn jwt_matches_rfc_7515_hs256_rs256_and_es256_vectors() {
        let historical_leeway = jsonwebtoken::get_current_timestamp();
        let payload = "eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";

        let hs256 = format!(
            "eyJ0eXAiOiJKV1QiLA0KICJhbGciOiJIUzI1NiJ9.{payload}.dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        );
        let hs256_key = [
            3, 35, 53, 75, 43, 15, 165, 188, 131, 126, 6, 101, 119, 123, 166, 143, 90, 179, 40,
            230, 240, 84, 201, 40, 169, 15, 132, 178, 210, 80, 46, 191, 211, 251, 90, 146, 210, 6,
            71, 239, 150, 138, 180, 195, 119, 98, 61, 34, 61, 46, 33, 114, 5, 46, 79, 8, 192, 205,
            154, 245, 103, 208, 128, 163,
        ];
        let hs256_options = JwtVerifyOptions {
            clock_tolerance: historical_leeway,
            ..JwtVerifyOptions::default()
        };
        assert!(
            verify_jwt(&hs256, &hs256_key, &hs256_options)
                .unwrap()
                .contains(r#""iss":"joe""#)
        );

        let rs256 = format!(
            "eyJhbGciOiJSUzI1NiJ9.{payload}.cC4hiUPoj9Eetdgtv3hF80EGrhuB__dzERat0XF9g2VtQgr9PJbu3XOiZj5RZmh7AAuHIm4Bh-0Qc_lF5YKt_O8W2Fp5jujGbds9uJdbF9CUAr7t1dnZcAcQjbKBYNX4BAynRFdiuB--f_nZLgrnbyTyWzO75vRK5h6xBArLIARNPvkSjtQBMHlb1L07Qe7K0GarZRmB_eSN9383LcOLn6_dO--xi12jzDwusC-eOkHWEsqtFZESc6BfI7noOPqvhJ1phCnvWh6IeYI2w9QOYEUipUTI8np6LbgGY9Fs98rqVt5AXLIhWkWywlVmtVrBp0igcN_IoypGlUPQGe77Rw"
        );
        let rs256_public_jwk = br#"{"kty":"RSA","n":"ofgWCuLjybRlzo0tZWJjNiuSfb4p4fAkd_wWJcyQoTbji9k0l8W26mPddxHmfHQp-Vaw-4qPCJrcS2mJPMEzP1Pt0Bm4d4QlL-yRT-SFd2lZS-pCgNMsD1W_YpRPEwOWvG6b32690r2jZ47soMZo9wGzjb_7OMg0LOL-bSf63kpaSHSXndS5z5rexMdbBYUsLA9e-KXBdQOS-UTo7WTBEMa2R2CapHg665xsmtdVMTBQY4uDZlxvb3qCo5ZwKh9kG4LT6_I5IhlJH7aGhyxXFvUK-DWNmoudF8NAco9_h9iaGNj8q2ethFkMLs91kzk2PAcDTW9gb54h4FRWyuXpoQ","e":"AQAB","alg":"RS256","use":"sig"}"#;
        let rs256_options = JwtVerifyOptions {
            algorithms: vec![jsonwebtoken::Algorithm::RS256],
            clock_tolerance: historical_leeway,
            ..JwtVerifyOptions::default()
        };
        assert!(
            verify_jwt(&rs256, rs256_public_jwk, &rs256_options)
                .unwrap()
                .contains(r#""exp":1300819380"#)
        );

        let es256 = format!(
            "eyJhbGciOiJFUzI1NiJ9.{payload}.DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSApmWQxfKTUJqPP3-Kg6NU1Q"
        );
        let es256_public_jwk = br#"{"kty":"EC","crv":"P-256","x":"f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU","y":"x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0","alg":"ES256","use":"sig"}"#;
        let es256_options = JwtVerifyOptions {
            algorithms: vec![jsonwebtoken::Algorithm::ES256],
            clock_tolerance: historical_leeway,
            ..JwtVerifyOptions::default()
        };
        assert!(
            verify_jwt(&es256, es256_public_jwk, &es256_options)
                .unwrap()
                .contains(r#""http://example.com/is_root":true"#)
        );
    }

    #[test]
    fn jwt_rejects_unsecured_and_unsupported_critical_headers() {
        let secret = b"0123456789abcdef0123456789abcdef";
        assert_eq!(
            verify_jwt(
                "eyJhbGciOiJub25lIn0.eyJpc3MiOiJqb2UifQ.",
                secret,
                &JwtVerifyOptions::default(),
            )
            .unwrap_err(),
            c"ERR_CRYPTO_JWT_MALFORMED"
        );

        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
        header.crit = Some(vec!["urn:example:must-understand".to_string()]);
        header.extras.insert(
            "urn:example:must-understand".to_string(),
            "required".to_string(),
        );
        let token = jsonwebtoken::encode(
            &header,
            &serde_json::json!({"sub": "user-42"}),
            &jsonwebtoken::EncodingKey::from_secret(secret),
        )
        .unwrap();
        assert_eq!(
            verify_jwt(&token, secret, &JwtVerifyOptions::default()).unwrap_err(),
            c"ERR_CRYPTO_JWT_UNSUPPORTED_CRITICAL_HEADER"
        );
        assert_eq!(
            verify_jwt_with_jwks(&token, r#"{"keys":[]}"#, &JwtVerifyOptions::default())
                .unwrap_err(),
            c"ERR_CRYPTO_JWT_UNSUPPORTED_CRITICAL_HEADER"
        );

        header.crit = Some(Vec::new());
        header.extras.clear();
        let empty_critical = jsonwebtoken::encode(
            &header,
            &serde_json::json!({"sub": "user-42"}),
            &jsonwebtoken::EncodingKey::from_secret(secret),
        )
        .unwrap();
        assert_eq!(
            verify_jwt(&empty_critical, secret, &JwtVerifyOptions::default()).unwrap_err(),
            c"ERR_CRYPTO_JWT_UNSUPPORTED_CRITICAL_HEADER"
        );
    }

    #[test]
    fn jwt_rejects_unsupported_algorithms_durations_and_oversized_keys() {
        assert_eq!(
            parse_jwt_algorithm("none").unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_ALGORITHM"
        );
        assert_eq!(
            parse_jwt_algorithm("ES512").unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_ALGORITHM"
        );
        assert!(parse_jwt_duration("0s").is_err());
        assert!(parse_jwt_duration("366d").is_err());
        let oversized = vec![b'x'; MAX_JWT_SECRET_BYTES + 1];
        assert_eq!(
            jwt_encoding_key(jsonwebtoken::Algorithm::RS256, &oversized).unwrap_err(),
            c"ERR_CRYPTO_JWT_INPUT_TOO_LARGE"
        );
        let symmetric_jwk = br#"{"kty":"oct","k":"MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY","alg":"HS256","use":"sig"}"#;
        assert_eq!(
            jwt_decoding_key(jsonwebtoken::Algorithm::HS256, symmetric_jwk).unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_KEY"
        );
        let private_jwk =
            br#"{"kty":"RSA","n":"AQAB","e":"AQAB","d":"AQAB","alg":"RS256","use":"sig"}"#;
        assert_eq!(
            jwt_decoding_key(jsonwebtoken::Algorithm::RS256, private_jwk).unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_KEY"
        );
    }

    #[test]
    fn jwt_supports_allowlisted_hmac_algorithms_and_claim_constraints() {
        let secret = b"0123456789abcdef0123456789abcdef";
        let sign_options = JwtSignOptions {
            algorithm: Some(jsonwebtoken::Algorithm::HS384),
            audience: Some("tyrus-api".to_string()),
            issuer: Some("https://issuer.example".to_string()),
            subject: Some("user-42".to_string()),
            expires_in: Some(60),
            ..JwtSignOptions::default()
        };
        let now = jsonwebtoken::get_current_timestamp();
        let token = sign_jwt("{}", secret, now, &sign_options).unwrap();
        let verify_options = JwtVerifyOptions {
            algorithms: vec![jsonwebtoken::Algorithm::HS384],
            audience: Some("tyrus-api".to_string()),
            issuer: Some("https://issuer.example".to_string()),
            subject: Some("user-42".to_string()),
            ..JwtVerifyOptions::default()
        };
        assert!(
            verify_jwt(&token, secret, &verify_options)
                .unwrap()
                .contains(r#""sub":"user-42""#)
        );
        assert_eq!(
            verify_jwt(&token, secret, &JwtVerifyOptions::default()).unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_ALGORITHM"
        );
    }

    #[test]
    fn jwt_round_trips_every_supported_algorithm_with_standard_pem_keys() {
        fn round_trip(algorithm: jsonwebtoken::Algorithm, private_key: &[u8], public_key: &[u8]) {
            let sign_options = JwtSignOptions {
                algorithm: Some(algorithm),
                no_timestamp: true,
                ..JwtSignOptions::default()
            };
            let token = sign_jwt(
                r#"{"sub":"algorithm-matrix"}"#,
                private_key,
                1,
                &sign_options,
            )
            .unwrap_or_else(|error| panic!("{algorithm:?} signing failed: {error:?}"));
            let verify_options = JwtVerifyOptions {
                algorithms: vec![algorithm],
                ..JwtVerifyOptions::default()
            };
            let claims = verify_jwt(&token, public_key, &verify_options)
                .unwrap_or_else(|error| panic!("{algorithm:?} verification failed: {error:?}"));
            assert!(
                claims.contains(r#""sub":"algorithm-matrix""#),
                "{algorithm:?} returned unexpected claims"
            );
        }

        let secret = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        for algorithm in [
            jsonwebtoken::Algorithm::HS256,
            jsonwebtoken::Algorithm::HS384,
            jsonwebtoken::Algorithm::HS512,
        ] {
            round_trip(algorithm, secret, secret);
        }
        let rsa_private = include_bytes!("../../tests/keys/rsa-private.pem");
        let rsa_public = include_bytes!("../../tests/keys/rsa-public.pem");
        for algorithm in [
            jsonwebtoken::Algorithm::RS256,
            jsonwebtoken::Algorithm::RS384,
            jsonwebtoken::Algorithm::RS512,
        ] {
            round_trip(algorithm, rsa_private, rsa_public);
        }
        round_trip(
            jsonwebtoken::Algorithm::ES256,
            include_bytes!("../../tests/keys/es256-private.pem"),
            include_bytes!("../../tests/keys/es256-public.pem"),
        );
        round_trip(
            jsonwebtoken::Algorithm::ES384,
            include_bytes!("../../tests/keys/es384-private.pem"),
            include_bytes!("../../tests/keys/es384-public.pem"),
        );
        round_trip(
            jsonwebtoken::Algorithm::EdDSA,
            include_bytes!("../../tests/keys/eddsa-private.pem"),
            include_bytes!("../../tests/keys/eddsa-public.pem"),
        );
    }

    #[test]
    fn jwt_rejects_missing_or_stale_claims_and_applies_clock_tolerance() {
        let secret = b"0123456789abcdef0123456789abcdef";
        let now = jsonwebtoken::get_current_timestamp();
        let unconstrained = sign_jwt("{}", secret, now, &JwtSignOptions::default()).unwrap();
        for options in [
            JwtVerifyOptions {
                audience: Some("tyrus-api".to_string()),
                ..JwtVerifyOptions::default()
            },
            JwtVerifyOptions {
                issuer: Some("https://issuer.example".to_string()),
                ..JwtVerifyOptions::default()
            },
            JwtVerifyOptions {
                subject: Some("user-42".to_string()),
                ..JwtVerifyOptions::default()
            },
        ] {
            assert_eq!(
                verify_jwt(&unconstrained, secret, &options).unwrap_err(),
                c"ERR_CRYPTO_JWT_INVALID_CLAIMS"
            );
        }

        let recently_expired = sign_jwt(
            &format!(r#"{{"exp":{}}}"#, now.saturating_sub(5)),
            secret,
            now,
            &JwtSignOptions::default(),
        )
        .unwrap();
        let tolerant = JwtVerifyOptions {
            clock_tolerance: 10,
            ..JwtVerifyOptions::default()
        };
        assert!(verify_jwt(&recently_expired, secret, &tolerant).is_ok());
        assert_eq!(
            verify_jwt(&recently_expired, secret, &JwtVerifyOptions::default()).unwrap_err(),
            c"ERR_CRYPTO_JWT_EXPIRED"
        );

        let without_timestamp = sign_jwt(
            "{}",
            secret,
            now,
            &JwtSignOptions {
                no_timestamp: true,
                ..JwtSignOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            verify_jwt(
                &without_timestamp,
                secret,
                &JwtVerifyOptions {
                    max_age: Some(60),
                    ..JwtVerifyOptions::default()
                }
            )
            .unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_CLAIMS"
        );
    }

    #[test]
    fn jwt_applies_rfc_7519_audience_and_not_before_semantics() {
        let secret = b"0123456789abcdef0123456789abcdef";
        let now = jsonwebtoken::get_current_timestamp();
        let token = sign_jwt(
            &format!(
                r#"{{"aud":["secondary-service","tyrus-api"],"nbf":{},"sub":"user-42"}}"#,
                now.saturating_add(30)
            ),
            secret,
            now,
            &JwtSignOptions::default(),
        )
        .unwrap();
        let strict = JwtVerifyOptions {
            audience: Some("tyrus-api".to_string()),
            subject: Some("user-42".to_string()),
            ..JwtVerifyOptions::default()
        };
        assert_eq!(
            verify_jwt(&token, secret, &strict).unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_CLAIMS"
        );
        let tolerant = JwtVerifyOptions {
            audience: Some("tyrus-api".to_string()),
            clock_tolerance: 30,
            subject: Some("user-42".to_string()),
            ..JwtVerifyOptions::default()
        };
        assert!(verify_jwt(&token, secret, &tolerant).is_ok());
    }

    #[test]
    fn jwks_rejects_missing_oversized_and_metadata_incompatible_kids() {
        let secret = b"0123456789abcdef0123456789abcdef";
        let now = jsonwebtoken::get_current_timestamp();
        let without_kid = sign_jwt("{}", secret, now, &JwtSignOptions::default()).unwrap();
        let empty_set = r#"{"keys":[]}"#;
        assert_eq!(
            verify_jwt_with_jwks(&without_kid, empty_set, &JwtVerifyOptions::default())
                .unwrap_err(),
            c"ERR_CRYPTO_JWT_MISSING_KID"
        );

        let oversized_kid = sign_jwt(
            "{}",
            secret,
            now,
            &JwtSignOptions {
                key_id: Some("k".repeat(257)),
                ..JwtSignOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            verify_jwt_with_jwks(&oversized_kid, empty_set, &JwtVerifyOptions::default())
                .unwrap_err(),
            c"ERR_CRYPTO_JWT_MISSING_KID"
        );

        let token = sign_jwt(
            "{}",
            secret,
            now,
            &JwtSignOptions {
                key_id: Some("active-key".to_string()),
                ..JwtSignOptions::default()
            },
        )
        .unwrap();
        for (hostile, expected) in [
            (
                r#"{"keys":[{"kty":"oct","k":"MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY","alg":"HS256","use":"enc","kid":"active-key"}]}"#,
                c"ERR_CRYPTO_JWT_INVALID_KEY",
            ),
            (
                r#"{"keys":[{"kty":"oct","k":"MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY","alg":"HS256","key_ops":["sign"],"kid":"active-key"}]}"#,
                c"ERR_CRYPTO_JWT_INVALID_KEY",
            ),
            (
                r#"{"keys":[{"kty":"oct","k":"MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY","alg":"HS384","kid":"active-key"}]}"#,
                c"ERR_CRYPTO_JWT_INVALID_KEY",
            ),
        ] {
            assert_eq!(
                verify_jwt_with_jwks(&token, hostile, &JwtVerifyOptions::default()).unwrap_err(),
                expected
            );
        }
    }

    #[test]
    fn jwks_verification_requires_one_compatible_key_for_the_header_kid() {
        const RSA_MODULUS: &str = "t6PBl3hhT4x7wPVD3HDi8agv6uIWa4yMqrprGuDQt9Wsfyn4xCs0NWE3Xp2bhWR0m-s4gmXO4lZpf2K6QA2Gl-MsQ6BdHByw_z0l2HAq24eFXphK6xXkXP65rj5TWMTBv7O1d7UcYjgpmtUtN1AdqNEMhVC4umHN4JimJVnNs1THXI5rJcL8NLeP4gWP1C9xGs90ba0aHmj5rhFoeuulRUYz1zoWI6L-ND2sjnXBS5Fwk5gaV96d2nUx3l8P68Lu7ZXy42PNGqQyTrOLe_SvGU_vWiPkEhTzE6uPQLCteGBUBEeZk_RhlwZ1ETy0O_LKRjwdUOtNSj-nyVvdtFJmjw";
        let options = JwtSignOptions {
            algorithm: Some(jsonwebtoken::Algorithm::RS256),
            key_id: Some("active-key".to_string()),
            no_timestamp: true,
            ..JwtSignOptions::default()
        };
        let token = sign_jwt(
            "{}",
            include_bytes!("../../tests/keys/rsa-private.pem"),
            jsonwebtoken::get_current_timestamp(),
            &options,
        )
        .unwrap();
        let jwks = format!(
            r#"{{"keys":[{{"kty":"RSA","n":"{RSA_MODULUS}","e":"AQAB","alg":"RS256","use":"sig","kid":"active-key"}}]}}"#
        );
        let verify_options = JwtVerifyOptions {
            algorithms: vec![jsonwebtoken::Algorithm::RS256],
            ..JwtVerifyOptions::default()
        };
        assert!(verify_jwt_with_jwks(&token, &jwks, &verify_options).is_ok());
        let private_jwks = jwks.replacen(r#""e":"AQAB""#, r#""e":"AQAB","d":"AQAB""#, 1);
        assert_eq!(
            verify_jwt_with_jwks(&token, &private_jwks, &verify_options).unwrap_err(),
            c"ERR_CRYPTO_JWT_INVALID_KEY"
        );

        let algorithm = TyrusString::literal("RS256");
        let algorithm_pointers = [algorithm.borrow()];
        let algorithms = TyrusArray {
            data: algorithm_pointers.as_ptr().cast_mut().cast::<u8>(),
            length: 1,
            capacity: 1,
        };
        let ffi_options = TyrusJwtVerifyOptions {
            algorithms: &algorithms,
            audience: ptr::null(),
            clock_tolerance: ptr::null(),
            issuer: ptr::null(),
            max_age: ptr::null(),
            subject: ptr::null(),
        };
        // SAFETY: `token` outlives the calls it is lent to.
        let token_value = unsafe { TyrusString::lend(&token) }.expect("within the limit");
        // SAFETY: `jwks` outlives the call it is lent to.
        let jwks_ffi = unsafe { TyrusString::lend(&jwks) }.expect("within the limit");
        unsafe {
            let claims = tyrus_crypto_jwt_verify_with_jwks(
                token_value.borrow(),
                jwks_ffi.borrow(),
                &ffi_options,
            );
            assert!(
                !claims.is_null(),
                "JWKS FFI failed: {:?}",
                take_crypto_error()
            );
            assert_eq!(take_string(claims), "{}");
        }

        let duplicate = format!(
            r#"{{"keys":[{{"kty":"RSA","n":"{RSA_MODULUS}","e":"AQAB","alg":"RS256","kid":"active-key"}},{{"kty":"RSA","n":"{RSA_MODULUS}","e":"AQAB","alg":"RS256","kid":"active-key"}}]}}"#
        );
        assert_eq!(
            verify_jwt_with_jwks(&token, &duplicate, &verify_options).unwrap_err(),
            c"ERR_CRYPTO_JWKS_DUPLICATE_KID"
        );
    }

    #[test]
    fn fetched_jwks_validation_rejects_duplicate_or_non_verification_keys() {
        const RSA_MODULUS: &str = "t6PBl3hhT4x7wPVD3HDi8agv6uIWa4yMqrprGuDQt9Wsfyn4xCs0NWE3Xp2bhWR0m-s4gmXO4lZpf2K6QA2Gl-MsQ6BdHByw_z0l2HAq24eFXphK6xXkXP65rj5TWMTBv7O1d7UcYjgpmtUtN1AdqNEMhVC4umHN4JimJVnNs1THXI5rJcL8NLeP4gWP1C9xGs90ba0aHmj5rhFoeuulRUYz1zoWI6L-ND2sjnXBS5Fwk5gaV96d2nUx3l8P68Lu7ZXy42PNGqQyTrOLe_SvGU_vWiPkEhTzE6uPQLCteGBUBEeZk_RhlwZ1ETy0O_LKRjwdUOtNSj-nyVvdtFJmjw";
        let duplicate = format!(
            r#"{{"keys":[{{"kty":"RSA","n":"{RSA_MODULUS}","e":"AQAB","kid":"same"}},{{"kty":"RSA","n":"{RSA_MODULUS}","e":"AQAB","kid":"same"}}]}}"#
        );
        assert_eq!(
            validate_fetched_jwks(&duplicate),
            Err("ERR_CRYPTO_JWKS_DUPLICATE_KID".to_string())
        );
        let encryption_only = format!(
            r#"{{"keys":[{{"kty":"RSA","n":"{RSA_MODULUS}","e":"AQAB","use":"enc","kid":"key"}}]}}"#
        );
        assert_eq!(
            validate_fetched_jwks(&encryption_only),
            Err("ERR_CRYPTO_JWKS_INVALID".to_string())
        );
        let symmetric = r#"{"keys":[{"kty":"oct","k":"MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY","use":"sig","kid":"key"}]}"#;
        assert_eq!(
            validate_fetched_jwks(symmetric),
            Err("ERR_CRYPTO_JWKS_INVALID".to_string())
        );
        let private = format!(
            r#"{{"keys":[{{"kty":"RSA","n":"{RSA_MODULUS}","e":"AQAB","d":"AQAB","use":"sig","kid":"key"}}]}}"#
        );
        assert_eq!(
            validate_fetched_jwks(&private),
            Err("ERR_CRYPTO_JWKS_INVALID".to_string())
        );
    }

    #[tokio::test]
    async fn jwks_fetch_rejects_ip_literals_and_private_addresses_before_connecting() {
        assert_eq!(
            fetch_jwks("https://127.0.0.1/.well-known/jwks.json".to_string(), false,).await,
            Err("ERR_CRYPTO_JWKS_FORBIDDEN_ADDRESS".to_string())
        );
        assert!(is_forbidden_ip("10.0.0.1".parse().unwrap()));
        assert!(is_forbidden_ip("::1".parse().unwrap()));
        assert!(!is_forbidden_ip("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn jwks_force_refresh_bypasses_an_unexpired_cache_entry() {
        let url = "https://cache-test.invalid/jwks";
        jwks_cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(url.to_string(), (Instant::now(), "cached".to_string()));
        assert_eq!(cached_jwks(url, false), Some("cached".to_string()));
        assert_eq!(cached_jwks(url, true), None);
        jwks_cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(url);
    }

    #[test]
    fn jwks_single_flight_is_scoped_per_url() {
        let first = jwks_url_lock("https://first.invalid/jwks").unwrap();
        let same = jwks_url_lock("https://first.invalid/jwks").unwrap();
        let other = jwks_url_lock("https://other.invalid/jwks").unwrap();
        assert!(Arc::ptr_eq(&first, &same));
        let _first_guard = first.try_lock().unwrap();
        assert!(same.try_lock().is_err());
        assert!(other.try_lock().is_ok());
        let mut locks = jwks_fetch_locks()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        locks.remove("https://first.invalid/jwks");
        locks.remove("https://other.invalid/jwks");
    }

    #[test]
    fn crypto_error_slot_is_cleared_and_consumed_exactly_once() {
        tyrus_crypto_clear_error();
        assert!(tyrus_crypto_take_error().is_null());
        set_crypto_error(c"ERR_CRYPTO_TEST");
        unsafe {
            let error = tyrus_crypto_take_error();
            assert_eq!(error.as_str(), "ERR_CRYPTO_TEST");
        }
        assert!(tyrus_crypto_take_error().is_null());
    }

    #[test]
    fn random_outputs_respect_format_and_bounds() {
        unsafe {
            let uuid = take_string(tyrus_crypto_rand_uuidv4());
            assert_eq!(uuid::Uuid::parse_str(&uuid).unwrap().get_version_num(), 4);
            let uuid = take_string(tyrus_crypto_rand_uuidv7());
            assert_eq!(uuid::Uuid::parse_str(&uuid).unwrap().get_version_num(), 7);
            let first = take_string(tyrus_crypto_rand_ulid());
            let second = take_string(tyrus_crypto_rand_ulid());
            assert!(first < second);
            let mut empty = TyrusArray {
                data: ptr::null_mut(),
                length: usize::MAX,
                capacity: usize::MAX,
            };
            tyrus_crypto_rand_random_bytes(0, &mut empty);
            assert!(empty.data.is_null());
            assert_eq!((empty.length, empty.capacity), (0, 0));
            for _ in 0..128 {
                assert!((-3..=7).contains(&tyrus_crypto_rand_random_int(-3, 7)));
                let value = tyrus_crypto_rand_random_float();
                assert!((0.0..1.0).contains(&value));
            }
            assert_eq!(tyrus_crypto_rand_random_int(i64::MIN, i64::MIN), i64::MIN);
            assert_eq!(tyrus_crypto_rand_random_int(i64::MAX, i64::MAX), i64::MAX);
            for _ in 0..32 {
                let _ = tyrus_crypto_rand_random_int(i64::MIN, i64::MAX);
            }
        }
        assert_eq!(validate_random_bytes_length(MAX_RANDOM_BYTES), Ok(()));
        assert_eq!(
            validate_random_bytes_length(MAX_RANDOM_BYTES + 1),
            Err("ERR_CRYPTO_RANDOM_LIMIT")
        );
        assert_eq!(
            validate_random_int_range(7, -3),
            Err("ERR_CRYPTO_INVALID_RANGE")
        );
    }

    #[test]
    fn random_identifiers_remain_unique_in_large_batches() {
        let mut uuid_v4 = std::collections::HashSet::new();
        let mut uuid_v7 = std::collections::HashSet::new();
        let mut ulids = std::collections::HashSet::new();
        for _ in 0..1_024 {
            unsafe {
                let v4 = take_string(tyrus_crypto_rand_uuidv4());
                let v7 = take_string(tyrus_crypto_rand_uuidv7());
                let ulid = take_string(tyrus_crypto_rand_ulid());
                assert_eq!(uuid::Uuid::parse_str(&v4).unwrap().get_version_num(), 4);
                assert_eq!(uuid::Uuid::parse_str(&v7).unwrap().get_version_num(), 7);
                assert_eq!(ulid.len(), 26);
                assert!(uuid_v4.insert(v4));
                assert!(uuid_v7.insert(v7));
                assert!(ulids.insert(ulid));
            }
        }
    }

    #[test]
    fn ulid_generation_is_unique_under_concurrency() {
        let values = (0..8)
            .map(|_| {
                std::thread::spawn(|| {
                    (0..64)
                        .map(|_| unsafe { take_string(tyrus_crypto_rand_ulid()) })
                        .collect::<Vec<_>>()
                })
            })
            .flat_map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>();
        let unique = values.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(unique.len(), values.len());
    }
}
