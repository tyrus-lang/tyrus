use crate::crypto::{
    argon2_hash_needs_rehash, bytes_from_raw, decrypt_aes_bytes, validate_fetched_jwks,
};

pub fn phc(data: &[u8]) {
    if let Ok(value) = std::str::from_utf8(data) {
        let _ = argon2_hash_needs_rehash(value);
    }
}

pub fn jwt_and_jwk(data: &[u8]) {
    if data.len() > 1024 * 1024 {
        return;
    }
    if let Ok(value) = std::str::from_utf8(data) {
        let _ = jsonwebtoken::dangerous::insecure_decode::<serde_json::Value>(value);
        if let Ok(set) = serde_json::from_str::<jsonwebtoken::jwk::JwkSet>(value) {
            for key in set.keys.iter().take(64) {
                let _ = jsonwebtoken::DecodingKey::from_jwk(key);
            }
        }
        let _ = validate_fetched_jwks(value);
    }
}

pub fn aes_envelope(data: &[u8]) {
    if data.len() > 1024 * 1024 {
        return;
    }
    let key = [0x5a; 32];
    let _ = decrypt_aes_bytes(data, &key, b"fuzz-aad");
}

pub fn utf8_and_ffi_bounds(data: &[u8]) {
    let _ = std::str::from_utf8(data);
    unsafe {
        let _ = bytes_from_raw(data.as_ptr(), data.len());
        let _ = bytes_from_raw(std::ptr::null(), 0);
        let _ = bytes_from_raw(std::ptr::null(), data.len().saturating_add(1));
    }
}
