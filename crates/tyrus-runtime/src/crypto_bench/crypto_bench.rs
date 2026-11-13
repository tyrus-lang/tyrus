use crate::abi::TyrusArray;
use crate::crypto::{
    tyrus_crypto_cipher_aes_decrypt_bytes, tyrus_crypto_cipher_aes_encrypt,
    tyrus_crypto_digest_sha256, tyrus_crypto_hmac_sha256,
};
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use tyrus_string::TyrusString;

static FFI_ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static FFI_ALLOCATION_BYTES: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn record_ffi_allocation(size: usize) {
    FFI_ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
    FFI_ALLOCATION_BYTES.fetch_add(size, Ordering::Relaxed);
}

pub fn reset_ffi_allocation_counters() {
    FFI_ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    FFI_ALLOCATION_BYTES.store(0, Ordering::Relaxed);
}

pub fn ffi_allocation_counters() -> (usize, usize) {
    (
        FFI_ALLOCATION_COUNT.load(Ordering::Relaxed),
        FFI_ALLOCATION_BYTES.load(Ordering::Relaxed),
    )
}

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

unsafe fn take_array(array: TyrusArray) -> Vec<u8> {
    let value = if array.length == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(array.data, array.length).to_vec()
    };
    free(array.data.cast());
    value
}

pub fn sha256(data: &[u8]) -> Vec<u8> {
    let mut output = TyrusArray {
        data: ptr::null_mut(),
        length: 0,
        capacity: 0,
    };
    unsafe {
        tyrus_crypto_digest_sha256(data.as_ptr(), data.len(), &mut output);
        take_array(output)
    }
}

pub fn hmac_sha256(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut output = TyrusArray {
        data: ptr::null_mut(),
        length: 0,
        capacity: 0,
    };
    unsafe {
        tyrus_crypto_hmac_sha256(
            data.as_ptr(),
            data.len(),
            key.as_ptr(),
            key.len(),
            &mut output,
        );
        take_array(output)
    }
}

pub fn timing_safe_equal(left: &[u8], right: &[u8]) -> bool {
    unsafe {
        crate::crypto::tyrus_crypto_timing_safe_equal(
            left.as_ptr(),
            left.len(),
            right.as_ptr(),
            right.len(),
        )
    }
}

pub fn aes_round_trip(data: &[u8], aad: &[u8]) -> Vec<u8> {
    let key = [0x5a; 32];
    let mut encrypted = TyrusArray {
        data: ptr::null_mut(),
        length: 0,
        capacity: 0,
    };
    unsafe {
        tyrus_crypto_cipher_aes_encrypt(
            data.as_ptr(),
            data.len(),
            key.as_ptr(),
            key.len(),
            aad.as_ptr(),
            aad.len(),
            &mut encrypted,
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
        free(encrypted.data.cast());
        take_array(decrypted)
    }
}

pub fn aes_encrypt_one_shot(data: &[u8], aad: &[u8]) -> Vec<u8> {
    use aes_gcm::{
        Aes256Gcm, KeyInit, Nonce,
        aead::{Aead, Payload},
    };
    let cipher = Aes256Gcm::new_from_slice(&[0x5a; 32]).unwrap();
    cipher
        .encrypt(&Nonce::from([0x42; 12]), Payload { msg: data, aad })
        .unwrap()
}

pub fn aes_encrypt_in_place(data: &mut Vec<u8>, aad: &[u8]) {
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::AeadInOut};
    let cipher = Aes256Gcm::new_from_slice(&[0x5a; 32]).unwrap();
    cipher
        .encrypt_in_place(&Nonce::from([0x42; 12]), aad, data)
        .unwrap();
}

pub fn argon2id(password: &[u8]) -> String {
    use argon2::password_hash::{PasswordHasher, SaltString};
    let salt = SaltString::encode_b64(b"benchmark-salt!").unwrap();
    let params = argon2::Params::new(65_536, 3, 4, Some(32)).unwrap();
    let algorithm =
        argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    algorithm
        .hash_password(password, &salt)
        .unwrap()
        .to_string()
}

pub fn jwt_hs256_round_trip(payload: &str) -> String {
    let payload = crate::strings::alloc_string(payload);
    let secret = TyrusString::literal("0123456789abcdef0123456789abcdef");
    unsafe {
        let token =
            crate::crypto::tyrus_crypto_jwt_sign(payload.borrow(), secret.borrow(), ptr::null());
        assert!(!token.is_null());
        let verified =
            crate::crypto::tyrus_crypto_jwt_verify(token.borrow(), secret.borrow(), ptr::null());
        assert!(!verified.is_null());
        let result = verified.as_str().to_owned();
        token.release();
        verified.release();
        payload.release();
        result
    }
}
