use crate::ast::{ExprKind, Type};

pub fn typeof_name(ty: &Type) -> String {
    match ty {
        Type::Lit(expr) => match expr.kind() {
            ExprKind::StrLit(_) => "string".to_string(),
            ExprKind::RegexLit { .. } => typeof_name(&Type::Regex),
            ExprKind::IntLit(_) => "i32".to_string(),
            ExprKind::UIntLit(_) => "u128".to_string(),
            ExprKind::FloatLit(_) => "f64".to_string(),
            ExprKind::BoolLit(_) => "boolean".to_string(),
            ExprKind::Null | ExprKind::None => "never".to_string(),
            _ => "unknown".to_string(),
        },
        Type::Object(_) => "object".to_string(),
        Type::Function { .. } | Type::GenericFunction { .. } => "function".to_string(),
        Type::Named(name) => crate::module_loader::original_global_name(name).to_string(),
        Type::Array(element) => format!("{}[]", typeof_name(element)),
        Type::Optional(inner) => format!("{}?", typeof_name(inner)),
        Type::Readonly(inner) => typeof_name(inner),
        Type::Ref(inner) | Type::MutRef(inner) => typeof_name(inner),
        other => other.to_string(),
    }
}

pub fn matches_typeof_name(ty: &Type, name: &str) -> bool {
    typeof_name(ty) == name
}
