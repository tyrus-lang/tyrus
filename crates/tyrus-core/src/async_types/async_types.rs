use crate::ast::{Expr, ExprKind, Field, Type};
use crate::collections::HashMap;
use crate::span::Span;

fn field(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: true,
        optional: false,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

pub fn error_interfaces() -> HashMap<String, Vec<Field>> {
    [
        (
            "Error".to_string(),
            vec![field("message", Type::Str), field("code", Type::Str)],
        ),
        (
            "AggregateError".to_string(),
            vec![
                field("message", Type::Str),
                field(
                    "errors",
                    Type::Array(Box::new(Type::Named("Error".to_string()))),
                ),
            ],
        ),
    ]
    .into_iter()
    .collect()
}

pub fn owned_string_frame_method(method: &str) -> bool {
    matches!(
        method,
        "copy"
            | "charAt"
            | "slice"
            | "substring"
            | "trim"
            | "trimStart"
            | "trimEnd"
            | "toLowerCase"
            | "toUpperCase"
            | "concat"
            | "replace"
            | "repeat"
            | "padStart"
            | "padEnd"
            | "toSlugCase"
            | "toDashCase"
            | "toKebabCase"
            | "toPascalCase"
            | "toCamelCase"
            | "toSnakeCase"
            | "truncate"
            | "truncateEllipsis"
            | "stripHtml"
            | "escapeHtml"
            | "urlEncode"
            | "urlDecode"
    )
}

/// Whether an array method returns a newly allocated array the caller owns.
///
/// Methods that mutate in place (`push`, `sort`, `reverse`) hand back the same
/// buffer the receiver already owns, and freeing that would be a double free.
/// Methods returning a scalar (`length`, `indexOf`, `includes`) allocate
/// nothing.
///
/// `join` builds a fresh `string`, not an array, its result is `Type::Str`,
/// not `Type::Array`/`Type::Tuple`, so the call site widens its own type
/// filter to cover it too; the name only needs to be here.
///
/// `at`/`pop`/`shift`/`find`/`findLast` box a fresh `T?` (the compiler's
/// nullable-pointer representation), for any element type including a
/// `Copy` one, the call site picks the release plan per method, since
/// `pop`/`shift` genuinely remove the element (full ownership transfers to
/// the box) while `at`/`find`/`findLast` leave the array unchanged (the box
/// is a fresh allocation, but a `string` element inside it is still an alias
/// the array itself will free later, only the box may be released, never
/// what it points to).
pub fn array_method_allocates(method: &str) -> bool {
    matches!(
        method,
        "map"
            | "filter"
            | "zip"
            | "toSorted"
            | "partition"
            | "windows"
            | "scan"
            | "unique"
            | "uniqueBy"
            | "groupBy"
            | "cartesianProduct"
            | "transpose"
            | "slice"
            | "concat"
            | "flat"
            | "flatMap"
            | "toReversed"
            | "shuffle"
            | "sample"
            | "sortedBy"
            | "with"
            | "splice"
            | "values"
            | "splitAt"
            | "join"
            | "keys"
            | "entries"
            | "at"
            | "random"
            | "first"
            | "last"
            | "avg"
            | "avgBy"
            | "pop"
            | "shift"
            | "find"
            | "findLast"
    )
}

pub fn owned_copy_array_frame_method(method: &str) -> bool {
    matches!(method, "of" | "from" | "copy") || array_method_allocates(method)
}

pub fn owned_string_handle_method(method: &str) -> bool {
    matches!(method, "localAddress" | "peerAddress" | "text")
}

pub fn owned_array_handle_method(method: &str) -> bool {
    matches!(method, "bytes")
}

pub fn owned_string_array_frame_method(method: &str) -> bool {
    matches!(method, "split" | "copy") || array_method_allocates(method)
}

pub fn static_string_array_frame_literal(expression: &Expr) -> bool {
    matches!(
        expression.kind(),
        ExprKind::ArrayLit(values)
            if values.iter().all(|value| matches!(value.kind(), ExprKind::StrLit(_)))
    )
}
