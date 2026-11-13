//! The single source of truth for how a `Type` is classified as a number.
//!
//! The compiler and the code generator both need these answers, and when each
//! kept its own copy they drifted: the checker's bounds table had no `isize`
//! and no `usize`, so an out-of-range literal assigned to one of them reached
//! codegen unreported. Anything that classifies a numeric type, or names its
//! range, belongs here and nowhere else.

use crate::ast::Type;

pub fn is_integer_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::Isize
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::Usize
    )
}

pub fn is_unsigned_integer_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::Usize
    )
}

pub fn is_float_type(ty: &Type) -> bool {
    matches!(ty, Type::F32 | Type::F64 | Type::F128)
}

/// The inclusive range of `ty`, or `None` when `ty` is not an integer. The
/// upper bound is `None` for `u128`, whose maximum does not fit in `i128`.
pub fn integer_type_bounds(ty: &Type) -> Option<(i128, Option<i128>)> {
    Some(match ty {
        Type::I8 => (i128::from(i8::MIN), Some(i128::from(i8::MAX))),
        Type::I16 => (i128::from(i16::MIN), Some(i128::from(i16::MAX))),
        Type::I32 => (i128::from(i32::MIN), Some(i128::from(i32::MAX))),
        Type::I64 | Type::Isize => (i128::from(i64::MIN), Some(i128::from(i64::MAX))),
        Type::I128 => (i128::MIN, Some(i128::MAX)),
        Type::U8 => (0, Some(i128::from(u8::MAX))),
        Type::U16 => (0, Some(i128::from(u16::MAX))),
        Type::U32 => (0, Some(i128::from(u32::MAX))),
        Type::U64 | Type::Usize => (0, Some(i128::from(u64::MAX))),
        Type::U128 => (0, None),
        _ => return None,
    })
}

pub fn integer_type_bits(ty: &Type) -> Option<u32> {
    Some(match ty {
        Type::I8 | Type::U8 => 8,
        Type::I16 | Type::U16 => 16,
        Type::I32 | Type::U32 => 32,
        Type::I64 | Type::Isize | Type::U64 | Type::Usize => 64,
        Type::I128 | Type::U128 => 128,
        _ => return None,
    })
}

/// The type behind `i32.MIN` and friends, or `None` when `type_name` is not a
/// numeric type or `field` is not one of its bounds.
pub fn numeric_constant_type(type_name: &str, field: &str) -> Option<Type> {
    if !matches!(field, "MIN" | "MAX") {
        return None;
    }
    match type_name {
        "i8" => Some(Type::I8),
        "i16" => Some(Type::I16),
        "i32" => Some(Type::I32),
        "i64" => Some(Type::I64),
        "i128" => Some(Type::I128),
        "u8" => Some(Type::U8),
        "u16" => Some(Type::U16),
        "u32" => Some(Type::U32),
        "u64" => Some(Type::U64),
        "u128" => Some(Type::U128),
        "f32" => Some(Type::F32),
        "f64" => Some(Type::F64),
        "f128" => Some(Type::F128),
        "decimal" => Some(Type::Decimal),
        _ => None,
    }
}
