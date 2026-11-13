use crate::ast::{Expr, ExprKind, Field, Type};
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;
use crate::span::Span;

pub const UTF8_ENCODE_TARGET: &str = "__tyrus_encoding_utf8_encode";
pub const UTF8_DECODE_TARGET: &str = "__tyrus_encoding_utf8_decode";
pub const UTF8_DECODE_LOSSY_TARGET: &str = "__tyrus_encoding_utf8_decode_lossy";
pub const UTF16_ENCODE_TARGET: &str = "__tyrus_encoding_utf16_encode";
pub const UTF16_ENCODE_LE_TARGET: &str = "__tyrus_encoding_utf16_encode_le";
pub const UTF16_ENCODE_BE_TARGET: &str = "__tyrus_encoding_utf16_encode_be";
pub const UTF16_DECODE_TARGET: &str = "__tyrus_encoding_utf16_decode";
pub const UTF16_DECODE_LOSSY_TARGET: &str = "__tyrus_encoding_utf16_decode_lossy";
pub const UTF16_DECODE_LE_TARGET: &str = "__tyrus_encoding_utf16_decode_le";
pub const UTF16_DECODE_LE_LOSSY_TARGET: &str = "__tyrus_encoding_utf16_decode_le_lossy";
pub const UTF16_DECODE_BE_TARGET: &str = "__tyrus_encoding_utf16_decode_be";
pub const UTF16_DECODE_BE_LOSSY_TARGET: &str = "__tyrus_encoding_utf16_decode_be_lossy";
pub const HEX_ENCODE_TARGET: &str = "__tyrus_encoding_hex_encode";
pub const HEX_DECODE_TARGET: &str = "__tyrus_encoding_hex_decode";
pub const BASE64_ENCODE_TARGET: &str = "__tyrus_encoding_base64_encode";
pub const BASE64_DECODE_TARGET: &str = "__tyrus_encoding_base64_decode";
pub const BASE64URL_ENCODE_TARGET: &str = "__tyrus_encoding_base64url_encode";
pub const BASE64URL_DECODE_TARGET: &str = "__tyrus_encoding_base64url_decode";
pub const BASE32_ENCODE_TARGET: &str = "__tyrus_encoding_base32_encode";
pub const BASE32_DECODE_TARGET: &str = "__tyrus_encoding_base32_decode";
pub const RADIX_FORMAT_TARGET: &str = "__tyrus_encoding_radix_format";
pub const RADIX_PARSE_I64_TARGET: &str = "__tyrus_encoding_radix_parse_i64";
pub const RADIX_PARSE_U64_TARGET: &str = "__tyrus_encoding_radix_parse_u64";

fn field(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: false,
        optional: false,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

fn u8_array() -> Type {
    Type::Array(Box::new(Type::U8))
}

fn u16_array() -> Type {
    Type::Array(Box::new(Type::U16))
}

fn encoding_result(success: Type) -> Type {
    Type::Result {
        success: Box::new(success),
        error: Box::new(Type::Named("EncodingError".to_string())),
    }
}

pub fn utf8_namespace_fields() -> Vec<Field> {
    vec![
        field(
            "encode",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(u8_array()),
            },
        ),
        field(
            "decode",
            Type::Function {
                params: vec![u8_array()],
                ret: Box::new(encoding_result(Type::Str)),
            },
        ),
        field(
            "decodeLossy",
            Type::Function {
                params: vec![u8_array()],
                ret: Box::new(Type::Str),
            },
        ),
    ]
}

pub fn utf16_namespace_fields() -> Vec<Field> {
    vec![
        field(
            "encode",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(u16_array()),
            },
        ),
        field(
            "encodeLE",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(u8_array()),
            },
        ),
        field(
            "encodeBE",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(u8_array()),
            },
        ),
        field(
            "decode",
            Type::Function {
                params: vec![u16_array()],
                ret: Box::new(encoding_result(Type::Str)),
            },
        ),
        field(
            "decodeLossy",
            Type::Function {
                params: vec![u16_array()],
                ret: Box::new(Type::Str),
            },
        ),
        field(
            "decodeLE",
            Type::Function {
                params: vec![u8_array()],
                ret: Box::new(encoding_result(Type::Str)),
            },
        ),
        field(
            "decodeLELossy",
            Type::Function {
                params: vec![u8_array()],
                ret: Box::new(Type::Str),
            },
        ),
        field(
            "decodeBE",
            Type::Function {
                params: vec![u8_array()],
                ret: Box::new(encoding_result(Type::Str)),
            },
        ),
        field(
            "decodeBELossy",
            Type::Function {
                params: vec![u8_array()],
                ret: Box::new(Type::Str),
            },
        ),
    ]
}

pub fn hex_namespace_fields() -> Vec<Field> {
    vec![
        field(
            "encode",
            Type::Function {
                params: vec![u8_array()],
                ret: Box::new(Type::Str),
            },
        ),
        field(
            "decode",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(encoding_result(u8_array())),
            },
        ),
    ]
}

pub fn base64_namespace_fields() -> Vec<Field> {
    vec![
        field(
            "encode",
            Type::Function {
                params: vec![u8_array()],
                ret: Box::new(Type::Str),
            },
        ),
        field(
            "decode",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(encoding_result(u8_array())),
            },
        ),
    ]
}

pub fn base64_url_namespace_fields() -> Vec<Field> {
    vec![
        field(
            "encode",
            Type::Function {
                params: vec![u8_array()],
                ret: Box::new(Type::Str),
            },
        ),
        field(
            "decode",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(encoding_result(u8_array())),
            },
        ),
    ]
}

pub fn base32_namespace_fields() -> Vec<Field> {
    vec![
        field(
            "encode",
            Type::Function {
                params: vec![u8_array()],
                ret: Box::new(Type::Str),
            },
        ),
        field(
            "decode",
            Type::Function {
                params: vec![Type::Str],
                ret: Box::new(encoding_result(u8_array())),
            },
        ),
    ]
}

pub fn radix_base_type() -> Type {
    Type::Union(
        [2, 8, 10, 16]
            .into_iter()
            .map(|base| Type::Lit(Box::new(Expr::synthetic(ExprKind::IntLit(base)))))
            .collect(),
    )
}

pub fn radix_namespace_fields() -> Vec<Field> {
    vec![
        field(
            "format",
            Type::Function {
                params: vec![Type::I64, radix_base_type()],
                ret: Box::new(Type::Str),
            },
        ),
        field(
            "parseI64",
            Type::Function {
                params: vec![Type::Str, radix_base_type()],
                ret: Box::new(encoding_result(Type::I64)),
            },
        ),
        field(
            "parseU64",
            Type::Function {
                params: vec![Type::Str, radix_base_type()],
                ret: Box::new(encoding_result(Type::U64)),
            },
        ),
    ]
}

pub fn encoding_target_returns_result(target: &str) -> bool {
    matches!(
        target,
        UTF8_DECODE_TARGET
            | UTF16_DECODE_TARGET
            | UTF16_DECODE_LE_TARGET
            | UTF16_DECODE_BE_TARGET
            | HEX_DECODE_TARGET
            | BASE64_DECODE_TARGET
            | BASE64URL_DECODE_TARGET
            | BASE32_DECODE_TARGET
            | RADIX_PARSE_I64_TARGET
            | RADIX_PARSE_U64_TARGET
    )
}

pub fn encoding_method_target(namespace: &str, method: &str) -> Option<&'static str> {
    match (namespace, method) {
        ("utf8", "encode") => Some(UTF8_ENCODE_TARGET),
        ("utf8", "decode") => Some(UTF8_DECODE_TARGET),
        ("utf8", "decodeLossy") => Some(UTF8_DECODE_LOSSY_TARGET),
        ("utf16", "encode") => Some(UTF16_ENCODE_TARGET),
        ("utf16", "encodeLE") => Some(UTF16_ENCODE_LE_TARGET),
        ("utf16", "encodeBE") => Some(UTF16_ENCODE_BE_TARGET),
        ("utf16", "decode") => Some(UTF16_DECODE_TARGET),
        ("utf16", "decodeLossy") => Some(UTF16_DECODE_LOSSY_TARGET),
        ("utf16", "decodeLE") => Some(UTF16_DECODE_LE_TARGET),
        ("utf16", "decodeLELossy") => Some(UTF16_DECODE_LE_LOSSY_TARGET),
        ("utf16", "decodeBE") => Some(UTF16_DECODE_BE_TARGET),
        ("utf16", "decodeBELossy") => Some(UTF16_DECODE_BE_LOSSY_TARGET),
        ("hex", "encode") => Some(HEX_ENCODE_TARGET),
        ("hex", "decode") => Some(HEX_DECODE_TARGET),
        ("base64", "encode") => Some(BASE64_ENCODE_TARGET),
        ("base64", "decode") => Some(BASE64_DECODE_TARGET),
        ("base64Url", "encode") => Some(BASE64URL_ENCODE_TARGET),
        ("base64Url", "decode") => Some(BASE64URL_DECODE_TARGET),
        ("base32", "encode") => Some(BASE32_ENCODE_TARGET),
        ("base32", "decode") => Some(BASE32_DECODE_TARGET),
        ("radix", "format") => Some(RADIX_FORMAT_TARGET),
        ("radix", "parseI64") => Some(RADIX_PARSE_I64_TARGET),
        ("radix", "parseU64") => Some(RADIX_PARSE_U64_TARGET),
        _ => None,
    }
}

pub fn encoding_namespace_interface_fields() -> Vec<(String, Vec<Field>)> {
    vec![
        ("Utf8Namespace".to_string(), utf8_namespace_fields()),
        ("Utf16Namespace".to_string(), utf16_namespace_fields()),
        ("HexNamespace".to_string(), hex_namespace_fields()),
        ("Base64Namespace".to_string(), base64_namespace_fields()),
        (
            "Base64UrlNamespace".to_string(),
            base64_url_namespace_fields(),
        ),
        ("Base32Namespace".to_string(), base32_namespace_fields()),
        ("RadixNamespace".to_string(), radix_namespace_fields()),
    ]
}

pub(crate) fn encoding_builtins() -> HashMap<String, ExportInfo> {
    [
        (
            "EncodingError".to_string(),
            ExportInfo {
                target: "EncodingError".to_string(),
                ty: Type::Named("EncodingError".to_string()),
                interface_fields: Some(crate::builtin_interfaces::encoding_error_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "utf8".to_string(),
            ExportInfo {
                target: "utf8".to_string(),
                ty: Type::Named("Utf8Namespace".to_string()),
                interface_fields: Some(utf8_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "utf16".to_string(),
            ExportInfo {
                target: "utf16".to_string(),
                ty: Type::Named("Utf16Namespace".to_string()),
                interface_fields: Some(utf16_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "hex".to_string(),
            ExportInfo {
                target: "hex".to_string(),
                ty: Type::Named("HexNamespace".to_string()),
                interface_fields: Some(hex_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "base64".to_string(),
            ExportInfo {
                target: "base64".to_string(),
                ty: Type::Named("Base64Namespace".to_string()),
                interface_fields: Some(base64_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "base64Url".to_string(),
            ExportInfo {
                target: "base64Url".to_string(),
                ty: Type::Named("Base64UrlNamespace".to_string()),
                interface_fields: Some(base64_url_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "base32".to_string(),
            ExportInfo {
                target: "base32".to_string(),
                ty: Type::Named("Base32Namespace".to_string()),
                interface_fields: Some(base32_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "radix".to_string(),
            ExportInfo {
                target: "radix".to_string(),
                ty: Type::Named("RadixNamespace".to_string()),
                interface_fields: Some(radix_namespace_fields()),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: Vec::new(),
                db_model: None,
            },
        ),
    ]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallible_encoding_methods_return_result_without_throw_effects() {
        for (fields, names) in [
            (utf8_namespace_fields(), vec!["decode"]),
            (
                utf16_namespace_fields(),
                vec!["decode", "decodeLE", "decodeBE"],
            ),
            (hex_namespace_fields(), vec!["decode"]),
            (base64_namespace_fields(), vec!["decode"]),
            (base64_url_namespace_fields(), vec!["decode"]),
            (base32_namespace_fields(), vec!["decode"]),
            (radix_namespace_fields(), vec!["parseI64", "parseU64"]),
        ] {
            for name in names {
                let field = fields.iter().find(|field| field.name == name).unwrap();
                let Type::Function { ret, .. } = &field.ty else {
                    panic!("encoding method `{name}` is not callable");
                };
                assert!(matches!(ret.as_ref(), Type::Result { .. }));
            }
        }
    }
}
