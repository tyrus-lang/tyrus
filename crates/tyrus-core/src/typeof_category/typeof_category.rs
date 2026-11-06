use crate::ast::{ExprKind, Type};

pub const TYPEOF_CATEGORIES: &[&str] = &[
    "string", "boolean", "char", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32",
    "u64", "u128", "usize", "f32", "f64", "f128", "decimal", "object", "function", "void", "never",
    "unknown",
];

const UNKNOWN_RUNTIME_VARIANTS: &[Type] = &[
    Type::Str,
    Type::Bool,
    Type::Char,
    Type::I8,
    Type::I16,
    Type::I32,
    Type::I64,
    Type::I128,
    Type::Isize,
    Type::U8,
    Type::U16,
    Type::U32,
    Type::U64,
    Type::U128,
    Type::Usize,
    Type::F32,
    Type::F64,
    Type::F128,
    Type::Decimal,
];

pub fn typeof_category(ty: &Type) -> &'static str {
    match ty {
        Type::Str => "string",
        Type::Bool => "boolean",
        Type::Char => "char",
        Type::I8 => "i8",
        Type::I16 => "i16",
        Type::I32 => "i32",
        Type::I64 => "i64",
        Type::I128 => "i128",
        Type::Isize => "isize",
        Type::U8 => "u8",
        Type::U16 => "u16",
        Type::U32 => "u32",
        Type::U64 => "u64",
        Type::U128 => "u128",
        Type::Usize => "usize",
        Type::F32 => "f32",
        Type::F64 => "f64",
        Type::F128 => "f128",
        Type::Decimal => "decimal",
        Type::Void => "void",
        Type::Never => "never",
        Type::Regex | Type::Optional(_) | Type::Array(_) | Type::Object(_) | Type::Tuple(_) => {
            "object"
        }
        Type::Function { .. } | Type::GenericFunction { .. } => "function",
        Type::Lit(literal) => match literal.kind() {
            ExprKind::StrLit(_) => "string",
            ExprKind::BoolLit(_) => "boolean",
            ExprKind::IntLit(_) => "i32",
            ExprKind::UIntLit(_) => "u128",
            ExprKind::FloatLit(_) => "f64",
            ExprKind::RegexLit { .. } => "object",
            _ => "unknown",
        },
        _ => "unknown",
    }
}

pub fn is_typeof_category(name: &str) -> bool {
    TYPEOF_CATEGORIES.contains(&name)
}

pub fn typeof_category_type(name: &str) -> Option<Type> {
    UNKNOWN_RUNTIME_VARIANTS
        .iter()
        .find(|variant| typeof_category(variant) == name)
        .cloned()
}

pub fn unknown_runtime_variants() -> Vec<Type> {
    UNKNOWN_RUNTIME_VARIANTS.to_vec()
}

pub fn is_unknown_runtime_union(ty: &Type) -> bool {
    matches!(ty, Type::Union(variants) if variants.as_slice() == UNKNOWN_RUNTIME_VARIANTS)
}

pub fn is_unknown_representable(ty: &Type) -> bool {
    match ty {
        Type::Unknown | Type::Never => true,
        Type::Union(variants) => variants.iter().all(is_unknown_representable),
        Type::Lit(_) => {
            let category = typeof_category(ty);
            typeof_category_type(category).is_some()
        }
        _ => UNKNOWN_RUNTIME_VARIANTS.contains(ty),
    }
}

pub fn typeof_category_suggestion(name: &str) -> Option<&'static str> {
    if let Some(category) = TYPEOF_CATEGORIES
        .iter()
        .find(|category| category.eq_ignore_ascii_case(name))
    {
        return Some(category);
    }
    match name.to_ascii_lowercase().as_str() {
        "bool" => Some("boolean"),
        "str" => Some("string"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_unknown_variant_round_trips_through_its_category() {
        for variant in UNKNOWN_RUNTIME_VARIANTS {
            let category = typeof_category(variant);
            assert!(is_typeof_category(category), "`{category}` is not listed");
            assert_eq!(typeof_category_type(category).as_ref(), Some(variant));
            assert!(is_unknown_representable(variant));
        }
    }

    #[test]
    fn unknown_variants_have_distinct_categories() {
        for (index, variant) in UNKNOWN_RUNTIME_VARIANTS.iter().enumerate() {
            for other in &UNKNOWN_RUNTIME_VARIANTS[index + 1..] {
                assert_ne!(typeof_category(variant), typeof_category(other));
            }
        }
    }

    #[test]
    fn composite_categories_prove_no_single_type() {
        for category in ["object", "function", "void", "never", "unknown"] {
            assert!(is_typeof_category(category));
            assert_eq!(typeof_category_type(category), None);
        }
        assert_eq!(typeof_category_type("number"), None);
    }

    #[test]
    fn composite_values_are_not_representable_in_unknown() {
        assert!(!is_unknown_representable(&Type::Array(Box::new(Type::I32))));
        assert!(!is_unknown_representable(&Type::Optional(Box::new(
            Type::Str
        ))));
        assert!(!is_unknown_representable(&Type::Named("User".to_string())));
        assert!(is_unknown_representable(&Type::Union(vec![
            Type::Str,
            Type::I32
        ])));
        assert!(!is_unknown_representable(&Type::Union(vec![
            Type::Str,
            Type::Array(Box::new(Type::I32)),
        ])));
    }

    #[test]
    fn suggestions_only_answer_unambiguous_spellings() {
        assert_eq!(typeof_category_suggestion("String"), Some("string"));
        assert_eq!(typeof_category_suggestion("BOOL"), Some("boolean"));
        assert_eq!(typeof_category_suggestion("str"), Some("string"));
        assert_eq!(typeof_category_suggestion("number"), None);
        assert_eq!(typeof_category_suggestion("strng"), None);
    }

    #[test]
    fn lowered_union_is_recognized_only_with_the_exact_variant_list() {
        assert!(is_unknown_runtime_union(&Type::Union(
            unknown_runtime_variants()
        )));
        assert!(!is_unknown_runtime_union(&Type::Union(vec![
            Type::Str,
            Type::Bool
        ])));
    }
}
