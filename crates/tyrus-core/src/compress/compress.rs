use crate::ast::*;
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;

pub const COMPRESS_TARGET: &str = "__tyrus_compress_compress";
pub const DECOMPRESS_TARGET: &str = "__tyrus_compress_decompress";
pub const COMPRESSION_STREAM_TARGET: &str = "__tyrus_compress_compression_stream";
pub const DECOMPRESSION_STREAM_TARGET: &str = "__tyrus_compress_decompression_stream";

/// `(name, ordinal, lowest level, highest level)`. Ordinals are part of the
/// ABI: the runtime reads them as the format.
pub const COMPRESS_FORMATS: [(&str, i128, i128, i128); 4] = [
    ("Gzip", 0, 0, 9),
    ("Deflate", 1, 0, 9),
    ("DeflateRaw", 2, 0, 9),
    ("Brotli", 3, 0, 11),
];

pub fn compress_format_enum() -> ResolvedEnum {
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members: COMPRESS_FORMATS
            .iter()
            .map(|(name, ordinal, _, _)| {
                (
                    (*name).to_string(),
                    Expr::synthetic(ExprKind::IntLit(*ordinal)),
                )
            })
            .collect(),
        member_kinds: vec![EnumPayload::Unit; COMPRESS_FORMATS.len()],
        is_algebraic: false,
    }
}

fn bytes() -> Type {
    Type::Array(Box::new(Type::U8))
}

/// Every function resolves with a `Result`: failures are values of
/// `CompressError`, never rejections.
fn promise_of_bytes_result() -> Type {
    Type::Apply {
        name: "Promise".to_string(),
        args: vec![Type::Result {
            success: Box::new(bytes()),
            error: Box::new(Type::Named("CompressError".to_string())),
        }],
    }
}

pub fn byte_transform_stream_type() -> Type {
    Type::Apply {
        name: "TransformStream".to_string(),
        args: vec![bytes(), bytes()],
    }
}

fn optional_named(name: &str) -> Type {
    Type::Optional(Box::new(Type::Named(name.to_string())))
}

/// The declared type of each function, shared by the export table and by the
/// codegen fallbacks that only know the dispatch target.
pub fn compress_target_type(target: &str) -> Option<Type> {
    let ty = match target {
        COMPRESS_TARGET => Type::Function {
            params: vec![
                Type::Union(vec![bytes(), Type::Str]),
                Type::Named("CompressFormat".to_string()),
                optional_named("CompressOptions"),
            ],
            ret: Box::new(promise_of_bytes_result()),
        },
        DECOMPRESS_TARGET => Type::Function {
            params: vec![bytes(), optional_named("DecompressOptions")],
            ret: Box::new(promise_of_bytes_result()),
        },
        COMPRESSION_STREAM_TARGET => Type::Function {
            params: vec![
                Type::Named("CompressFormat".to_string()),
                optional_named("CompressOptions"),
            ],
            ret: Box::new(byte_transform_stream_type()),
        },
        DECOMPRESSION_STREAM_TARGET => Type::Function {
            params: vec![optional_named("DecompressOptions")],
            ret: Box::new(byte_transform_stream_type()),
        },
        _ => return None,
    };
    Some(ty)
}

fn export(
    target: &str,
    ty: Type,
    interface_fields: Option<Vec<Field>>,
    enum_info: Option<ResolvedEnum>,
) -> ExportInfo {
    ExportInfo {
        target: target.to_string(),
        ty,
        interface_fields,
        behavior_info: None,
        type_alias_ty: None,
        enum_info,
        throws: Vec::new(),
        db_model: None,
    }
}

fn function_export(target: &str) -> ExportInfo {
    export(
        target,
        compress_target_type(target).expect("every compress target has a type"),
        None,
        None,
    )
}

fn interface_export(name: &str, fields: Vec<Field>) -> ExportInfo {
    export(name, Type::Named(name.to_string()), Some(fields), None)
}

pub(crate) fn compress_builtins() -> HashMap<String, ExportInfo> {
    use crate::builtin_interfaces::{
        compress_error_fields, compress_options_fields, decompress_options_fields,
    };
    [
        ("compress".to_string(), function_export(COMPRESS_TARGET)),
        ("decompress".to_string(), function_export(DECOMPRESS_TARGET)),
        (
            "compressionStream".to_string(),
            function_export(COMPRESSION_STREAM_TARGET),
        ),
        (
            "decompressionStream".to_string(),
            function_export(DECOMPRESSION_STREAM_TARGET),
        ),
        (
            "CompressFormat".to_string(),
            export(
                "CompressFormat",
                Type::Named("CompressFormat".to_string()),
                None,
                Some(compress_format_enum()),
            ),
        ),
        (
            "CompressError".to_string(),
            interface_export("CompressError", compress_error_fields()),
        ),
        (
            "CompressOptions".to_string(),
            interface_export("CompressOptions", compress_options_fields()),
        ),
        (
            "DecompressOptions".to_string(),
            interface_export("DecompressOptions", decompress_options_fields()),
        ),
    ]
    .into_iter()
    .collect()
}
