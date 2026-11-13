use crate::ast::*;
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;
use crate::span::Span;

pub const TIMING_SAFE_EQUAL_TARGET: &str = "__tyrus_crypto_timing_safe_equal";
pub const HMAC_SHA256_TARGET: &str = "__tyrus_crypto_hmac_sha256";
pub const HMAC_SHA384_TARGET: &str = "__tyrus_crypto_hmac_sha384";
pub const HMAC_SHA512_TARGET: &str = "__tyrus_crypto_hmac_sha512";
pub const DIGEST_SHA256_TARGET: &str = "__tyrus_crypto_digest_sha256";
pub const DIGEST_SHA384_TARGET: &str = "__tyrus_crypto_digest_sha384";
pub const DIGEST_SHA512_TARGET: &str = "__tyrus_crypto_digest_sha512";
pub const DIGEST_MD5_TARGET: &str = "__tyrus_crypto_digest_md5";
pub const RAND_UUIDV7_TARGET: &str = "__tyrus_crypto_rand_uuidv7";
pub const RAND_UUIDV4_TARGET: &str = "__tyrus_crypto_rand_uuidv4";
pub const RAND_ULID_TARGET: &str = "__tyrus_crypto_rand_ulid";
pub const RAND_RANDOM_BYTES_TARGET: &str = "__tyrus_crypto_rand_random_bytes";
pub const RAND_RANDOM_INT_TARGET: &str = "__tyrus_crypto_rand_random_int";
pub const RAND_RANDOM_FLOAT_TARGET: &str = "__tyrus_crypto_rand_random_float";
pub const CIPHER_AES_ENCRYPT_TARGET: &str = "__tyrus_crypto_cipher_aes_encrypt";
pub const CIPHER_AES_DECRYPT_STR_TARGET: &str = "__tyrus_crypto_cipher_aes_decrypt_str";
pub const CIPHER_AES_DECRYPT_BYTES_TARGET: &str = "__tyrus_crypto_cipher_aes_decrypt_bytes";
pub const ARGON2_HASH_TARGET: &str = "__tyrus_crypto_argon2_hash";
pub const ARGON2_VERIFY_TARGET: &str = "__tyrus_crypto_argon2_verify";
pub const ARGON2_NEEDS_REHASH_TARGET: &str = "__tyrus_crypto_argon2_needs_rehash";
pub const JWT_SIGN_TARGET: &str = "__tyrus_crypto_jwt_sign";
pub const JWT_VERIFY_TARGET: &str = "__tyrus_crypto_jwt_verify";
pub const JWT_DECODE_TARGET: &str = "__tyrus_crypto_jwt_decode";
pub const JWT_FETCH_JWKS_TARGET: &str = "__tyrus_crypto_jwt_fetch_jwks";
pub const JWT_VERIFY_WITH_JWKS_TARGET: &str = "__tyrus_crypto_jwt_verify_with_jwks";
pub const DIGEST_CREATE_TARGET: &str = "__tyrus_crypto_digest_create";
pub const DIGEST_STREAM_UPDATE_TARGET: &str = "__tyrus_crypto_digest_stream_update";
pub const DIGEST_STREAM_DIGEST_TARGET: &str = "__tyrus_crypto_digest_stream_digest";
pub const DIGEST_STREAM_RESET_TARGET: &str = "__tyrus_crypto_digest_stream_reset";
pub const DIGEST_STREAM_DISPOSE_TARGET: &str = "__tyrus_crypto_digest_stream_dispose";

fn str_or_bytes_type() -> Type {
    Type::Union(vec![Type::Str, Type::Array(Box::new(Type::U8))])
}

fn u8_array_type() -> Type {
    Type::Array(Box::new(Type::U8))
}

fn crypto_result(success: Type) -> Type {
    Type::Result {
        success: Box::new(success),
        error: Box::new(Type::Named("CryptoError".to_string())),
    }
}

fn promise_type(inner: Type) -> Type {
    Type::Apply {
        name: "Promise".to_string(),
        args: vec![inner],
    }
}

fn crypto_field(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: false,
        optional: false,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

fn crypto_optional_field(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: false,
        optional: true,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

pub fn argon2_hash_options_type() -> Type {
    Type::Object(vec![
        crypto_optional_field("memoryCost", Type::U32),
        crypto_optional_field("parallelism", Type::U32),
        crypto_optional_field("secret", u8_array_type()),
        crypto_optional_field("timeCost", Type::U32),
        crypto_optional_field("type", Type::Str),
    ])
}

pub fn argon2_verify_options_type() -> Type {
    Type::Object(vec![crypto_optional_field("secret", u8_array_type())])
}

pub fn jwt_sign_options_type() -> Type {
    Type::Object(vec![
        crypto_optional_field("algorithm", Type::Str),
        crypto_optional_field("audience", Type::Str),
        crypto_optional_field("expiresIn", Type::Str),
        crypto_optional_field("issuer", Type::Str),
        crypto_optional_field("jwtId", Type::Str),
        crypto_optional_field("keyId", Type::Str),
        crypto_optional_field("noTimestamp", Type::Bool),
        crypto_optional_field("subject", Type::Str),
    ])
}

pub fn jwt_verify_options_type() -> Type {
    Type::Object(vec![
        crypto_optional_field("algorithms", Type::Array(Box::new(Type::Str))),
        crypto_optional_field("audience", Type::Str),
        crypto_optional_field("clockTolerance", Type::U32),
        crypto_optional_field("issuer", Type::Str),
        crypto_optional_field("maxAge", Type::Str),
        crypto_optional_field("subject", Type::Str),
    ])
}

pub fn jwks_fetch_options_type() -> Type {
    Type::Object(vec![crypto_optional_field("forceRefresh", Type::Bool)])
}

pub fn digest_stream_methods() -> Vec<Field> {
    vec![
        crypto_field(
            "update",
            Type::Function {
                params: vec![str_or_bytes_type()],
                ret: Box::new(crypto_result(Type::Named("DigestStream".to_string()))),
            },
        ),
        crypto_field(
            "digest",
            Type::Function {
                params: vec![],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
        crypto_field(
            "reset",
            Type::Function {
                params: vec![],
                ret: Box::new(crypto_result(Type::Named("DigestStream".to_string()))),
            },
        ),
        crypto_field(
            "dispose",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Void),
            },
        ),
    ]
}

pub fn digest_stream_method_target(method: &str) -> Option<&'static str> {
    match method {
        "update" => Some(DIGEST_STREAM_UPDATE_TARGET),
        "digest" => Some(DIGEST_STREAM_DIGEST_TARGET),
        "reset" => Some(DIGEST_STREAM_RESET_TARGET),
        "dispose" => Some(DIGEST_STREAM_DISPOSE_TARGET),
        _ => None,
    }
}

pub(crate) fn crypto_builtins() -> HashMap<String, ExportInfo> {
    let mut builtins = HashMap::default();
    for (name, info) in crypto_root_builtins() {
        builtins.insert(name, info);
    }
    for (name, info) in crypto_namespace_exports() {
        builtins.insert(name, info);
    }
    builtins
}

pub(crate) fn crypto_root_builtins() -> Vec<(String, ExportInfo)> {
    vec![(
        "timingSafeEqual".to_string(),
        ExportInfo {
            target: TIMING_SAFE_EQUAL_TARGET.to_string(),
            ty: Type::GenericFunction {
                type_params: vec![],
                params: vec![str_or_bytes_type(), str_or_bytes_type()],
                ret: Box::new(Type::Bool),
            },
            interface_fields: None,
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    )]
}

pub fn hmac_namespace_fields() -> Vec<Field> {
    vec![
        crypto_field(
            "sha256",
            Type::Function {
                params: vec![str_or_bytes_type(), str_or_bytes_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
        crypto_field(
            "sha384",
            Type::Function {
                params: vec![str_or_bytes_type(), str_or_bytes_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
        crypto_field(
            "sha512",
            Type::Function {
                params: vec![str_or_bytes_type(), str_or_bytes_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
    ]
}

pub fn digest_namespace_fields() -> Vec<Field> {
    vec![
        crypto_field(
            "create",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(crypto_result(Type::Named("DigestStream".to_string()))),
            },
        ),
        crypto_field(
            "sha256",
            Type::Function {
                params: vec![str_or_bytes_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
        crypto_field(
            "sha384",
            Type::Function {
                params: vec![str_or_bytes_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
        crypto_field(
            "sha512",
            Type::Function {
                params: vec![str_or_bytes_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
        crypto_field(
            "md5",
            Type::Function {
                params: vec![str_or_bytes_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
    ]
}

pub fn cipher_namespace_fields() -> Vec<Field> {
    vec![crypto_field(
        "aes",
        Type::Named("CipherAesNamespace".to_string()),
    )]
}

pub fn cipher_aes_namespace_fields() -> Vec<Field> {
    vec![
        crypto_field(
            "encrypt",
            Type::Function {
                params: vec![str_or_bytes_type(), u8_array_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
        crypto_field(
            "encryptWithAad",
            Type::Function {
                params: vec![str_or_bytes_type(), u8_array_type(), u8_array_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
        crypto_field(
            "decryptText",
            Type::Function {
                params: vec![u8_array_type(), u8_array_type()],
                ret: Box::new(crypto_result(Type::Str)),
            },
        ),
        crypto_field(
            "decryptBytes",
            Type::Function {
                params: vec![u8_array_type(), u8_array_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
        crypto_field(
            "decryptTextWithAad",
            Type::Function {
                params: vec![u8_array_type(), u8_array_type(), u8_array_type()],
                ret: Box::new(crypto_result(Type::Str)),
            },
        ),
        crypto_field(
            "decryptBytesWithAad",
            Type::Function {
                params: vec![u8_array_type(), u8_array_type(), u8_array_type()],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
    ]
}

pub fn rand_namespace_fields() -> Vec<Field> {
    vec![
        crypto_field(
            "uuidv7",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Str),
            },
        ),
        crypto_field(
            "uuidv4",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Str),
            },
        ),
        crypto_field(
            "ulid",
            Type::Function {
                params: vec![],
                ret: Box::new(crypto_result(Type::Str)),
            },
        ),
        crypto_field(
            "randomBytes",
            Type::Function {
                params: vec![Type::U32],
                ret: Box::new(crypto_result(u8_array_type())),
            },
        ),
        crypto_field(
            "randomInt",
            Type::Function {
                params: vec![Type::I64, Type::I64],
                ret: Box::new(crypto_result(Type::I64)),
            },
        ),
        crypto_field(
            "randomFloat",
            Type::Function {
                params: vec![],
                ret: Box::new(Type::F64),
            },
        ),
    ]
}

pub fn argon2_namespace_fields() -> Vec<Field> {
    vec![
        crypto_field("argon2id", Type::Str),
        crypto_field(
            "hash",
            Type::Function {
                params: vec![
                    str_or_bytes_type(),
                    Type::Optional(Box::new(argon2_hash_options_type())),
                ],
                ret: Box::new(promise_type(crypto_result(Type::Str))),
            },
        ),
        crypto_field(
            "verify",
            Type::Function {
                params: vec![
                    Type::Str,
                    str_or_bytes_type(),
                    Type::Optional(Box::new(argon2_verify_options_type())),
                ],
                ret: Box::new(promise_type(crypto_result(Type::Bool))),
            },
        ),
        crypto_field(
            "needsRehash",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(crypto_result(Type::Bool)),
            },
        ),
    ]
}

pub fn crypto_namespace_constant(namespace_type: &str, field: &str) -> Option<&'static str> {
    match (namespace_type, field) {
        ("Argon2Namespace", "argon2id") => Some("argon2id"),
        _ => None,
    }
}

pub fn crypto_method_parameter_names(
    namespace: &str,
    method: &str,
) -> Option<&'static [&'static str]> {
    match (namespace, method) {
        ("digest", "create") => Some(&["algorithm"]),
        ("digest", "sha256" | "sha384" | "sha512" | "md5") => Some(&["data"]),
        ("hmac", "sha256" | "sha384" | "sha512") => Some(&["data", "key"]),
        ("cipher.aes", "encrypt" | "decryptText" | "decryptBytes") => Some(&["data", "key"]),
        ("cipher.aes", "encryptWithAad" | "decryptTextWithAad" | "decryptBytesWithAad") => {
            Some(&["data", "key", "aad"])
        }
        ("rand", "uuidv7" | "uuidv4" | "ulid" | "randomFloat") => Some(&[]),
        ("rand", "randomBytes") => Some(&["length"]),
        ("rand", "randomInt") => Some(&["min", "max"]),
        ("argon2", "hash") => Some(&["password", "options"]),
        ("argon2", "verify") => Some(&["hash", "password", "options"]),
        ("argon2", "needsRehash") => Some(&["hash"]),
        ("jwt", "sign") => Some(&["payload", "key", "options"]),
        ("jwt", "verify") => Some(&["token", "key", "options"]),
        ("jwt", "decodeUnverified") => Some(&["token"]),
        ("jwt", "fetchJwks") => Some(&["url", "options"]),
        ("jwt", "verifyWithJwks") => Some(&["token", "jwks", "options"]),
        ("DigestStream", "update") => Some(&["data"]),
        ("DigestStream", "digest" | "reset" | "dispose") => Some(&[]),
        _ => None,
    }
}

pub fn jwt_namespace_fields() -> Vec<Field> {
    let claim_type_parameter = TypeParam {
        name: "T".to_string(),
        constraint: None,
    };
    vec![
        crypto_field(
            "fetchJwks",
            Type::Function {
                params: vec![
                    Type::Str,
                    Type::Optional(Box::new(jwks_fetch_options_type())),
                ],
                ret: Box::new(promise_type(crypto_result(Type::Str))),
            },
        ),
        crypto_field(
            "sign",
            Type::GenericFunction {
                type_params: vec![claim_type_parameter.clone()],
                params: vec![
                    Type::Generic("T".to_string()),
                    Type::Str,
                    Type::Optional(Box::new(jwt_sign_options_type())),
                ],
                ret: Box::new(crypto_result(Type::Str)),
            },
        ),
        crypto_field(
            "verify",
            Type::GenericFunction {
                type_params: vec![claim_type_parameter.clone()],
                params: vec![
                    Type::Str,
                    Type::Str,
                    Type::Optional(Box::new(jwt_verify_options_type())),
                ],
                ret: Box::new(crypto_result(Type::Generic("T".to_string()))),
            },
        ),
        crypto_field(
            "verifyWithJwks",
            Type::GenericFunction {
                type_params: vec![claim_type_parameter.clone()],
                params: vec![
                    Type::Str,
                    Type::Str,
                    Type::Optional(Box::new(jwt_verify_options_type())),
                ],
                ret: Box::new(crypto_result(Type::Generic("T".to_string()))),
            },
        ),
        crypto_field(
            "decodeUnverified",
            Type::GenericFunction {
                type_params: vec![claim_type_parameter],
                params: vec![Type::Str],
                ret: Box::new(crypto_result(Type::Generic("T".to_string()))),
            },
        ),
    ]
}

pub(crate) fn crypto_namespace_exports() -> Vec<(String, ExportInfo)> {
    vec![
        (
            "CryptoError".to_string(),
            ExportInfo {
                target: "CryptoError".to_string(),
                ty: Type::Named("CryptoError".to_string()),
                interface_fields: Some(crate::builtin_interfaces::crypto_error_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        ),
        (
            "digest".to_string(),
            ExportInfo {
                target: "digest".to_string(),
                ty: Type::Named("DigestNamespace".to_string()),
                interface_fields: Some(digest_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        ),
        (
            "hmac".to_string(),
            ExportInfo {
                target: "hmac".to_string(),
                ty: Type::Named("HmacNamespace".to_string()),
                interface_fields: Some(hmac_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        ),
        (
            "cipher".to_string(),
            ExportInfo {
                target: "cipher".to_string(),
                ty: Type::Named("CipherNamespace".to_string()),
                interface_fields: Some(cipher_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        ),
        (
            "rand".to_string(),
            ExportInfo {
                target: "rand".to_string(),
                ty: Type::Named("RandNamespace".to_string()),
                interface_fields: Some(rand_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        ),
        (
            "argon2".to_string(),
            ExportInfo {
                target: "argon2".to_string(),
                ty: Type::Named("Argon2Namespace".to_string()),
                interface_fields: Some(argon2_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        ),
        (
            "jwt".to_string(),
            ExportInfo {
                target: "jwt".to_string(),
                ty: Type::Named("JwtNamespace".to_string()),
                interface_fields: Some(jwt_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        ),
    ]
}

pub fn crypto_method_target(namespace: &str, method: &str) -> Option<&'static str> {
    match (namespace, method) {
        ("hmac", "sha256") => Some(HMAC_SHA256_TARGET),
        ("hmac", "sha384") => Some(HMAC_SHA384_TARGET),
        ("hmac", "sha512") => Some(HMAC_SHA512_TARGET),
        ("digest", "sha256") => Some(DIGEST_SHA256_TARGET),
        ("digest", "sha384") => Some(DIGEST_SHA384_TARGET),
        ("digest", "sha512") => Some(DIGEST_SHA512_TARGET),
        ("digest", "md5") => Some(DIGEST_MD5_TARGET),
        ("digest", "create") => Some(DIGEST_CREATE_TARGET),
        ("rand", "uuidv7") => Some(RAND_UUIDV7_TARGET),
        ("rand", "uuidv4") => Some(RAND_UUIDV4_TARGET),
        ("rand", "ulid") => Some(RAND_ULID_TARGET),
        ("rand", "randomBytes") => Some(RAND_RANDOM_BYTES_TARGET),
        ("rand", "randomInt") => Some(RAND_RANDOM_INT_TARGET),
        ("rand", "randomFloat") => Some(RAND_RANDOM_FLOAT_TARGET),
        ("cipher", "aes") => None,
        ("argon2", "hash") => Some(ARGON2_HASH_TARGET),
        ("argon2", "verify") => Some(ARGON2_VERIFY_TARGET),
        ("argon2", "needsRehash") => Some(ARGON2_NEEDS_REHASH_TARGET),
        ("jwt", "sign") => Some(JWT_SIGN_TARGET),
        ("jwt", "verify") => Some(JWT_VERIFY_TARGET),
        ("jwt", "decodeUnverified") => Some(JWT_DECODE_TARGET),
        ("jwt", "fetchJwks") => Some(JWT_FETCH_JWKS_TARGET),
        ("jwt", "verifyWithJwks") => Some(JWT_VERIFY_WITH_JWKS_TARGET),
        _ => None,
    }
}

pub fn is_crypto_namespace(ty_name: &str) -> bool {
    matches!(
        ty_name,
        "HmacNamespace"
            | "DigestNamespace"
            | "CipherNamespace"
            | "CipherAesNamespace"
            | "RandNamespace"
            | "Argon2Namespace"
            | "JwtNamespace"
    )
}

pub fn crypto_type_to_import_name(ty_name: &str) -> Option<&'static str> {
    match ty_name {
        "DigestNamespace" => Some("digest"),
        "HmacNamespace" => Some("hmac"),
        "CipherNamespace" => Some("cipher"),
        "CipherAesNamespace" => Some("cipher"),
        "RandNamespace" => Some("rand"),
        "Argon2Namespace" => Some("argon2"),
        "JwtNamespace" => Some("jwt"),
        _ => None,
    }
}

pub fn nested_crypto_inner_type_name(outer_type_name: &str, field: &str) -> Option<&'static str> {
    match (outer_type_name, field) {
        ("CipherNamespace", "aes") => Some("CipherAesNamespace"),
        _ => None,
    }
}

pub fn nested_crypto_method_target(namespace: &str, method: &str) -> Option<&'static str> {
    match (namespace, method) {
        ("CipherAesNamespace", "encrypt") => Some(CIPHER_AES_ENCRYPT_TARGET),
        ("CipherAesNamespace", "encryptWithAad") => Some(CIPHER_AES_ENCRYPT_TARGET),
        ("CipherAesNamespace", "decryptText") => Some(CIPHER_AES_DECRYPT_STR_TARGET),
        ("CipherAesNamespace", "decryptBytes") => Some(CIPHER_AES_DECRYPT_BYTES_TARGET),
        ("CipherAesNamespace", "decryptTextWithAad") => Some(CIPHER_AES_DECRYPT_STR_TARGET),
        ("CipherAesNamespace", "decryptBytesWithAad") => Some(CIPHER_AES_DECRYPT_BYTES_TARGET),
        _ => None,
    }
}

pub fn method_returns_owned_string(target: &str) -> bool {
    matches!(
        target,
        RAND_UUIDV7_TARGET | RAND_UUIDV4_TARGET | RAND_ULID_TARGET | CIPHER_AES_DECRYPT_STR_TARGET
    )
}

pub fn method_returns_owned_claims(target: &str) -> bool {
    matches!(
        target,
        JWT_VERIFY_TARGET | JWT_VERIFY_WITH_JWKS_TARGET | JWT_DECODE_TARGET
    )
}

pub fn crypto_method_throws(namespace_type: &str, _method: &str) -> bool {
    match namespace_type {
        "DigestNamespace" | "DigestStream" => false,
        "RandNamespace" => false,
        "CipherAesNamespace" => false,
        "Argon2Namespace" => false,
        "JwtNamespace" => false,
        _ => false,
    }
}

pub fn crypto_namespace_interface_fields() -> Vec<(String, Vec<Field>)> {
    vec![
        ("DigestNamespace".to_string(), digest_namespace_fields()),
        ("HmacNamespace".to_string(), hmac_namespace_fields()),
        ("CipherNamespace".to_string(), cipher_namespace_fields()),
        (
            "CipherAesNamespace".to_string(),
            cipher_aes_namespace_fields(),
        ),
        ("RandNamespace".to_string(), rand_namespace_fields()),
        ("Argon2Namespace".to_string(), argon2_namespace_fields()),
        ("JwtNamespace".to_string(), jwt_namespace_fields()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallible_random_methods_return_crypto_error_results() {
        let builtin_interfaces = crate::builtin_interfaces::builtin_interfaces();
        assert_eq!(
            builtin_interfaces.get("CryptoError"),
            Some(&crate::builtin_interfaces::crypto_error_fields())
        );
        assert!(
            !crate::builtin_interfaces::ambient_builtin_interfaces().contains_key("CryptoError")
        );

        for name in ["ulid", "randomBytes", "randomInt"] {
            let field = rand_namespace_fields()
                .into_iter()
                .find(|field| field.name == name)
                .unwrap();
            let Type::Function { ret, .. } = field.ty else {
                panic!("rand.{name} is not callable");
            };
            assert!(matches!(
                ret.as_ref(),
                Type::Result { error, .. }
                    if matches!(error.as_ref(), Type::Named(type_name) if type_name == "CryptoError")
            ));
            assert!(!crypto_method_throws("RandNamespace", name));
        }
        let cases: [(&str, Vec<Field>, &[&str]); 2] = [
            (
                "HmacNamespace",
                hmac_namespace_fields(),
                &["sha256", "sha384", "sha512"],
            ),
            (
                "DigestNamespace",
                digest_namespace_fields(),
                &["sha256", "sha384", "sha512", "md5"],
            ),
        ];
        for (namespace, fields, names) in cases {
            for name in names {
                let field = fields.iter().find(|field| field.name == *name).unwrap();
                let Type::Function { ret, .. } = &field.ty else {
                    panic!("{namespace}.{name} is not callable");
                };
                assert!(matches!(
                    ret.as_ref(),
                    Type::Result { error, .. }
                        if matches!(error.as_ref(), Type::Named(type_name) if type_name == "CryptoError")
                ));
            }
        }
        for name in [
            "encrypt",
            "encryptWithAad",
            "decryptText",
            "decryptBytes",
            "decryptTextWithAad",
            "decryptBytesWithAad",
        ] {
            let field = cipher_aes_namespace_fields()
                .into_iter()
                .find(|field| field.name == name)
                .unwrap();
            let Type::Function { ret, .. } = field.ty else {
                panic!("cipher.aes.{name} is not callable");
            };
            assert!(matches!(
                ret.as_ref(),
                Type::Result { error, .. }
                    if matches!(error.as_ref(), Type::Named(type_name) if type_name == "CryptoError")
            ));
            assert!(!crypto_method_throws("CipherAesNamespace", name));
        }

        let sign = jwt_namespace_fields()
            .into_iter()
            .find(|field| field.name == "sign")
            .unwrap();
        let Type::GenericFunction { ret, .. } = sign.ty else {
            panic!("jwt.sign is not callable");
        };
        assert!(matches!(
            ret.as_ref(),
            Type::Result { success, error }
                if matches!(success.as_ref(), Type::Str)
                    && matches!(error.as_ref(), Type::Named(type_name) if type_name == "CryptoError")
        ));
        assert!(!crypto_method_throws("JwtNamespace", "sign"));

        let create = digest_namespace_fields()
            .into_iter()
            .find(|field| field.name == "create")
            .unwrap();
        assert!(matches!(
            create.ty,
            Type::Function { ret, .. }
                if matches!(ret.as_ref(), Type::Result { success, error }
                    if matches!(success.as_ref(), Type::Named(name) if name == "DigestStream")
                        && matches!(error.as_ref(), Type::Named(name) if name == "CryptoError"))
        ));
        assert!(!crypto_method_throws("DigestNamespace", "create"));
        for method in ["update", "digest", "reset"] {
            let field = digest_stream_methods()
                .into_iter()
                .find(|field| field.name == method)
                .unwrap();
            assert!(matches!(
                field.ty,
                Type::Function { ret, .. }
                    if matches!(ret.as_ref(), Type::Result { error, .. }
                        if matches!(error.as_ref(), Type::Named(name) if name == "CryptoError"))
            ));
            assert!(!crypto_method_throws("DigestStream", method));
        }

        for name in ["verify", "verifyWithJwks", "decodeUnverified"] {
            let field = jwt_namespace_fields()
                .into_iter()
                .find(|field| field.name == name)
                .unwrap();
            let Type::GenericFunction { ret, .. } = field.ty else {
                panic!("jwt.{name} is not callable");
            };
            assert!(matches!(
                ret.as_ref(),
                Type::Result { success, error }
                    if matches!(success.as_ref(), Type::Generic(type_name) if type_name == "T")
                        && matches!(error.as_ref(), Type::Named(type_name) if type_name == "CryptoError")
            ));
            assert!(!crypto_method_throws("JwtNamespace", name));
        }
    }
}
