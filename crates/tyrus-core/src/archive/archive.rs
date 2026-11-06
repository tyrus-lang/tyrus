use crate::ast::*;
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;

pub const PACK_TARGET: &str = "__tyrus_archive_pack";
pub const LIST_TARGET: &str = "__tyrus_archive_list";
pub const UNPACK_TARGET: &str = "__tyrus_archive_unpack";
pub const PACK_STREAM_TARGET: &str = "__tyrus_archive_pack_stream";
pub const UNPACK_STREAM_TARGET: &str = "__tyrus_archive_unpack_stream";

/// Ordinals are part of the ABI: the runtime reads them as the format a
/// stream is written in, since a stream has no extension to decide it.
pub fn archive_format_enum() -> ResolvedEnum {
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members: vec![
            ("Zip".to_string(), Expr::synthetic(ExprKind::IntLit(0))),
            ("Tar".to_string(), Expr::synthetic(ExprKind::IntLit(1))),
            ("TarGz".to_string(), Expr::synthetic(ExprKind::IntLit(2))),
        ],
        member_kinds: vec![EnumPayload::Unit; 3],
        is_algebraic: false,
    }
}

fn byte_stream() -> Type {
    Type::Apply {
        name: "ReadableStream".to_string(),
        args: vec![Type::Array(Box::new(Type::U8))],
    }
}

/// Ordinals are part of the ABI: the runtime writes them into `ArchiveEntry`.
pub fn archive_entry_kind_enum() -> ResolvedEnum {
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members: vec![
            ("File".to_string(), Expr::synthetic(ExprKind::IntLit(0))),
            (
                "Directory".to_string(),
                Expr::synthetic(ExprKind::IntLit(1)),
            ),
            ("Symlink".to_string(), Expr::synthetic(ExprKind::IntLit(2))),
            ("HardLink".to_string(), Expr::synthetic(ExprKind::IntLit(3))),
            ("Other".to_string(), Expr::synthetic(ExprKind::IntLit(4))),
        ],
        member_kinds: vec![EnumPayload::Unit; 5],
        is_algebraic: false,
    }
}

fn promise_of(ty: Type) -> Type {
    Type::Apply {
        name: "Promise".to_string(),
        args: vec![ty],
    }
}

/// Every archive function resolves with a `Result`: failures are values of
/// `ArchiveError`, never rejections.
fn promise_of_result(success: Type) -> Type {
    promise_of(Type::Result {
        success: Box::new(success),
        error: Box::new(Type::Named("ArchiveError".to_string())),
    })
}

fn optional_named(name: &str) -> Type {
    Type::Optional(Box::new(Type::Named(name.to_string())))
}

/// The declared type of each archive function, shared by the export table
/// and by the codegen fallbacks that only know the dispatch target.
pub fn archive_target_type(target: &str) -> Option<Type> {
    let ty = match target {
        PACK_TARGET => Type::Function {
            params: vec![Type::Str, Type::Str, optional_named("PackOptions")],
            ret: Box::new(promise_of_result(Type::Named("ArchiveSummary".to_string()))),
        },
        LIST_TARGET => Type::Function {
            params: vec![Type::Str, optional_named("UnpackOptions")],
            ret: Box::new(promise_of_result(Type::Array(Box::new(Type::Named(
                "ArchiveEntry".to_string(),
            ))))),
        },
        UNPACK_TARGET => Type::Function {
            params: vec![Type::Str, Type::Str, optional_named("UnpackOptions")],
            ret: Box::new(promise_of_result(Type::Named("ArchiveSummary".to_string()))),
        },
        PACK_STREAM_TARGET => Type::Function {
            params: vec![
                Type::Str,
                Type::Named("ArchiveFormat".to_string()),
                optional_named("PackOptions"),
            ],
            ret: Box::new(byte_stream()),
        },
        UNPACK_STREAM_TARGET => Type::Function {
            params: vec![byte_stream(), Type::Str, optional_named("UnpackOptions")],
            ret: Box::new(promise_of_result(Type::Named("ArchiveSummary".to_string()))),
        },
        _ => return None,
    };
    Some(ty)
}

fn function_export(target: &str) -> ExportInfo {
    ExportInfo {
        target: target.to_string(),
        ty: archive_target_type(target).expect("every archive target has a type"),
        interface_fields: None,
        behavior_info: None,
        type_alias_ty: None,
        enum_info: None,
        throws: Vec::new(),
        db_model: None,
    }
}

fn interface_export(name: &str, fields: Vec<Field>) -> ExportInfo {
    ExportInfo {
        target: name.to_string(),
        ty: Type::Named(name.to_string()),
        interface_fields: Some(fields),
        behavior_info: None,
        type_alias_ty: None,
        enum_info: None,
        throws: Vec::new(),
        db_model: None,
    }
}

pub(crate) fn archive_builtins() -> HashMap<String, ExportInfo> {
    use crate::builtin_interfaces::{
        archive_entry_fields, archive_error_fields, archive_summary_fields, pack_options_fields,
        unpack_options_fields,
    };
    [
        ("pack".to_string(), function_export(PACK_TARGET)),
        ("list".to_string(), function_export(LIST_TARGET)),
        ("unpack".to_string(), function_export(UNPACK_TARGET)),
        (
            "packStream".to_string(),
            function_export(PACK_STREAM_TARGET),
        ),
        (
            "unpackStream".to_string(),
            function_export(UNPACK_STREAM_TARGET),
        ),
        (
            "ArchiveFormat".to_string(),
            ExportInfo {
                target: "ArchiveFormat".to_string(),
                ty: Type::Named("ArchiveFormat".to_string()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: Some(archive_format_enum()),
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "ArchiveEntryKind".to_string(),
            ExportInfo {
                target: "ArchiveEntryKind".to_string(),
                ty: Type::Named("ArchiveEntryKind".to_string()),
                interface_fields: None,
                behavior_info: None,
                type_alias_ty: None,
                enum_info: Some(archive_entry_kind_enum()),
                throws: Vec::new(),
                db_model: None,
            },
        ),
        (
            "ArchiveEntry".to_string(),
            interface_export("ArchiveEntry", archive_entry_fields()),
        ),
        (
            "ArchiveError".to_string(),
            interface_export("ArchiveError", archive_error_fields()),
        ),
        (
            "ArchiveSummary".to_string(),
            interface_export("ArchiveSummary", archive_summary_fields()),
        ),
        (
            "PackOptions".to_string(),
            interface_export("PackOptions", pack_options_fields()),
        ),
        (
            "UnpackOptions".to_string(),
            interface_export("UnpackOptions", unpack_options_fields()),
        ),
    ]
    .into_iter()
    .collect()
}
