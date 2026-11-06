use crate::ast::{Field, Type};
use crate::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutomaticDataMethod {
    Copy,
    Debug,
    Eq,
    JsonEncode,
    JsonDecode,
}

impl AutomaticDataMethod {
    pub fn instance(name: &str) -> Option<Self> {
        match name {
            "copy" => Some(Self::Copy),
            "toString" => Some(Self::Debug),
            "equals" => Some(Self::Eq),
            "toJson" => Some(Self::JsonEncode),
            _ => None,
        }
    }

    pub fn static_method(name: &str) -> Option<Self> {
        match name {
            "fromJson" => Some(Self::JsonDecode),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Debug => "toString",
            Self::Eq => "equals",
            Self::JsonEncode => "toJson",
            Self::JsonDecode => "fromJson",
        }
    }

    pub const fn is_static(self) -> bool {
        matches!(self, Self::JsonDecode)
    }
}

pub fn supports_debug(
    ty: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
) -> bool {
    supports_debug_inner(ty, interfaces, data_interfaces, &mut HashSet::default())
}

pub fn supports_copy(
    ty: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
) -> bool {
    supports_copy_inner(ty, interfaces, data_interfaces, &mut HashSet::default())
}

fn supports_copy_inner(
    ty: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
    visiting: &mut HashSet<String>,
) -> bool {
    if ty.is_copy() || matches!(ty, Type::Str) {
        return true;
    }
    match ty {
        Type::Array(inner) | Type::Optional(inner) => {
            supports_copy_inner(inner, interfaces, data_interfaces, visiting)
        }
        Type::Tuple(elements) => elements
            .iter()
            .all(|element| supports_copy_inner(element, interfaces, data_interfaces, visiting)),
        Type::Object(fields) => fields
            .iter()
            .all(|field| supports_copy_inner(&field.ty, interfaces, data_interfaces, visiting)),
        Type::Named(name) => {
            if !data_interfaces.contains(name) || !visiting.insert(name.clone()) {
                return false;
            }
            let supported = interfaces.get(name).is_some_and(|fields| {
                fields.iter().all(|field| {
                    supports_copy_inner(&field.ty, interfaces, data_interfaces, visiting)
                })
            });
            visiting.remove(name);
            supported
        }
        _ => false,
    }
}

pub fn supports_eq(
    ty: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
) -> bool {
    supports_eq_inner(ty, interfaces, data_interfaces, &mut HashSet::default())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonDirection {
    Encode,
    Decode,
}

pub fn supports_json_encode(
    ty: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
) -> bool {
    supports_json_in(ty, interfaces, data_interfaces, JsonDirection::Encode)
}

pub fn supports_json(
    ty: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
) -> bool {
    supports_json_in(ty, interfaces, data_interfaces, JsonDirection::Decode)
}

fn supports_json_in(
    ty: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
    direction: JsonDirection,
) -> bool {
    let Type::Named(name) = ty else {
        return false;
    };
    if !data_interfaces.contains(name) {
        return false;
    }
    let mut visiting = HashSet::default();
    supports_json_named(name, interfaces, data_interfaces, &mut visiting, direction)
}

fn supports_json_named(
    name: &str,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
    visiting: &mut HashSet<String>,
    direction: JsonDirection,
) -> bool {
    if !data_interfaces.contains(name) || !visiting.insert(name.to_string()) {
        return false;
    }
    let supported = interfaces.get(name).is_some_and(|fields| {
        fields.iter().all(|field| {
            supports_json_inner(
                &field.ty,
                interfaces,
                data_interfaces,
                visiting,
                false,
                direction,
            )
        })
    });
    visiting.remove(name);
    supported
}

fn supports_json_inner(
    ty: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
    visiting: &mut HashSet<String>,
    inside_array: bool,
    direction: JsonDirection,
) -> bool {
    match ty {
        Type::Optional(inner) if !inside_array => supports_json_inner(
            inner,
            interfaces,
            data_interfaces,
            visiting,
            false,
            direction,
        ),
        Type::Array(inner) if !inside_array => supports_json_inner(
            inner,
            interfaces,
            data_interfaces,
            visiting,
            true,
            direction,
        ),
        Type::Named(name) => {
            supports_json_named(name, interfaces, data_interfaces, visiting, direction)
        }
        Type::Union(variants) => {
            is_discriminated_union(variants)
                && variants.iter().all(|variant| {
                    supports_json_variant(variant, interfaces, data_interfaces, visiting, direction)
                })
        }
        Type::Str
        | Type::Bool
        | Type::I32
        | Type::I64
        | Type::U32
        | Type::U64
        | Type::F32
        | Type::F64
        | Type::Decimal => true,
        _ => false,
    }
}

fn supports_json_variant(
    variant: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
    visiting: &mut HashSet<String>,
    direction: JsonDirection,
) -> bool {
    let Type::Object(fields) = variant else {
        return false;
    };
    fields.iter().all(|field| {
        if let Type::Lit(literal) = &field.ty {
            return json_literal_is_renderable(literal);
        }
        supports_json_inner(
            &field.ty,
            interfaces,
            data_interfaces,
            visiting,
            false,
            direction,
        )
    })
}

fn json_literal_is_renderable(literal: &crate::ast::Expr) -> bool {
    json_literal_key(literal).is_some()
}

fn json_literal_key(literal: &crate::ast::Expr) -> Option<String> {
    use crate::ast::ExprKind;
    Some(match literal.kind() {
        ExprKind::StrLit(text) => format!("s{text}"),
        ExprKind::IntLit(value) => format!("i{value}"),
        ExprKind::UIntLit(value) => format!("u{value}"),
        ExprKind::FloatLit(value) => format!("f{}", value.value()),
        ExprKind::BoolLit(value) => format!("b{value}"),
        _ => return None,
    })
}

pub fn union_discriminant_field(variants: &[Type]) -> Option<String> {
    if variants.len() < 2 {
        return None;
    }
    let Some(Type::Object(first)) = variants.first() else {
        return None;
    };
    first
        .iter()
        .find(|candidate| {
            matches!(candidate.ty, Type::Lit(_)) && discriminates(variants, &candidate.name)
        })
        .map(|field| field.name.clone())
}

pub fn union_variant_discriminant_text(variant: &Type, discriminant: &str) -> Option<String> {
    let Type::Object(fields) = variant else {
        return None;
    };
    let field = fields.iter().find(|field| field.name == discriminant)?;
    let Type::Lit(literal) = &field.ty else {
        return None;
    };
    use crate::ast::ExprKind;
    Some(match literal.kind() {
        ExprKind::StrLit(text) => text.clone(),
        ExprKind::IntLit(value) => value.to_string(),
        ExprKind::UIntLit(value) => value.to_string(),
        ExprKind::BoolLit(value) => value.to_string(),
        _ => return None,
    })
}

fn discriminates(variants: &[Type], name: &str) -> bool {
    let mut seen: Vec<String> = Vec::new();
    for variant in variants {
        let Type::Object(fields) = variant else {
            return false;
        };
        let Some(field) = fields.iter().find(|field| field.name == name) else {
            return false;
        };
        let Type::Lit(literal) = &field.ty else {
            return false;
        };
        let Some(written) = json_literal_key(literal) else {
            return false;
        };
        if seen.contains(&written) {
            return false;
        }
        seen.push(written);
    }
    true
}

pub fn is_discriminated_union(variants: &[Type]) -> bool {
    if variants.len() < 2 {
        return false;
    }
    let Some(Type::Object(first)) = variants.first() else {
        return false;
    };
    first.iter().any(|candidate| {
        matches!(candidate.ty, Type::Lit(_)) && discriminates(variants, &candidate.name)
    })
}

fn supports_eq_inner(
    ty: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
    visiting: &mut HashSet<String>,
) -> bool {
    match ty {
        Type::Optional(inner) => supports_eq_inner(inner, interfaces, data_interfaces, visiting),
        Type::Array(_) => false,
        Type::Tuple(elements) => elements
            .iter()
            .all(|element| supports_eq_inner(element, interfaces, data_interfaces, visiting)),
        Type::Object(fields) => fields
            .iter()
            .all(|field| supports_eq_inner(&field.ty, interfaces, data_interfaces, visiting)),
        Type::Named(name) => {
            if !data_interfaces.contains(name) || !visiting.insert(name.clone()) {
                return false;
            }
            let supported = interfaces.get(name).is_some_and(|fields| {
                fields.iter().all(|field| {
                    supports_eq_inner(&field.ty, interfaces, data_interfaces, visiting)
                })
            });
            visiting.remove(name);
            supported
        }
        _ => true,
    }
}

fn supports_debug_inner(
    ty: &Type,
    interfaces: &HashMap<String, Vec<Field>>,
    data_interfaces: &HashSet<String>,
    visiting: &mut HashSet<String>,
) -> bool {
    if matches!(
        ty,
        Type::Str
            | Type::Bool
            | Type::Char
            | Type::I8
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
            | Type::F32
            | Type::F64
            | Type::F128
            | Type::Decimal
    ) {
        return true;
    }
    match ty {
        Type::Object(fields) => fields
            .iter()
            .all(|field| supports_debug_inner(&field.ty, interfaces, data_interfaces, visiting)),
        Type::Optional(inner) | Type::Array(inner) => {
            supports_debug_inner(inner, interfaces, data_interfaces, visiting)
        }
        Type::Union(variants) | Type::Tuple(variants) => variants
            .iter()
            .all(|variant| supports_debug_inner(variant, interfaces, data_interfaces, visiting)),
        Type::Named(name) => {
            if !data_interfaces.contains(name) || !visiting.insert(name.clone()) {
                return false;
            }
            let supported = interfaces.get(name).is_some_and(|fields| {
                fields.iter().all(|field| {
                    supports_debug_inner(&field.ty, interfaces, data_interfaces, visiting)
                })
            });
            visiting.remove(name);
            supported
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{AutomaticDataMethod, supports_debug};
    use crate::ast::{Field, Type};
    use crate::collections::HashMap;

    #[test]
    fn recognizes_the_public_automatic_method_names() {
        assert_eq!(
            AutomaticDataMethod::instance("copy"),
            Some(AutomaticDataMethod::Copy)
        );
        assert_eq!(
            AutomaticDataMethod::instance("toString"),
            Some(AutomaticDataMethod::Debug)
        );
        assert_eq!(
            AutomaticDataMethod::instance("equals"),
            Some(AutomaticDataMethod::Eq)
        );
        assert_eq!(
            AutomaticDataMethod::instance("toJson"),
            Some(AutomaticDataMethod::JsonEncode)
        );
        assert_eq!(
            AutomaticDataMethod::static_method("fromJson"),
            Some(AutomaticDataMethod::JsonDecode)
        );
        assert_eq!(AutomaticDataMethod::instance("fromJson"), None);
        assert_eq!(AutomaticDataMethod::static_method("toJson"), None);
    }

    #[test]
    fn accepts_recursive_data_shapes_for_debug() {
        let mut interfaces = HashMap::default();
        interfaces.insert(
            "User".to_string(),
            vec![Field {
                name: "tags".to_string(),
                ty: Type::Array(Box::new(Type::Str)),
                readonly: false,
                optional: false,
                decorators: Vec::new(),
                span: crate::span::Span::DUMMY,
            }],
        );
        let data_interfaces = ["User".to_string()].into_iter().collect();
        assert!(supports_debug(
            &Type::Named("User".to_string()),
            &interfaces,
            &data_interfaces
        ));
    }
}
