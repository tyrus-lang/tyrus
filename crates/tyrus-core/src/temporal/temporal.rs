use crate::ast::*;
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;

#[path = "temporal_builtins/temporal_builtins.rs"]
mod temporal_builtins;

pub(crate) use temporal_builtins::temporal_builtins;

fn temporal_function(target: &str, params: Vec<Type>, ret: Type) -> ExportInfo {
    ExportInfo {
        target: target.to_string(),
        ty: Type::Function {
            params,
            ret: Box::new(ret),
        },
        interface_fields: None,
        behavior_info: None,
        type_alias_ty: None,
        enum_info: None,
        throws: Vec::new(),
        db_model: None,
    }
}

fn temporal_result_function(target: &str, params: Vec<Type>, success: Type) -> ExportInfo {
    let mut export = temporal_function(target, params, success.clone());
    let Type::Function { ret, .. } = &mut export.ty else {
        unreachable!("temporal_function always creates a function export");
    };
    **ret = Type::Result {
        success: Box::new(success),
        error: Box::new(Type::Named("TemporalError".to_string())),
    };
    export
}

#[cfg(test)]
fn time_zone_names() -> impl Iterator<Item = &'static str> {
    tyrus_temporal_data::TIME_ZONES
        .iter()
        .map(|zone| zone.member)
}

pub fn time_zone_enum() -> ResolvedEnum {
    let members: Vec<(String, Expr)> = tyrus_temporal_data::TIME_ZONES
        .iter()
        .enumerate()
        .map(|(ordinal, zone)| {
            (
                zone.member.to_string(),
                Expr::synthetic(ExprKind::IntLit(ordinal as i128)),
            )
        })
        .collect();
    let count = members.len();
    ResolvedEnum {
        repr: Type::I32,
        nominal: false,
        members,
        member_kinds: vec![EnumPayload::Unit; count],
        is_algebraic: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{temporal_builtins, time_zone_enum, time_zone_names};
    use crate::ast::Type;

    #[test]
    fn exposes_the_full_checked_in_timezone_catalog() {
        let names: Vec<_> = time_zone_names().collect();
        let timezone = time_zone_enum();

        assert!(names.len() > 500);
        assert_eq!(timezone.members.len(), names.len());
        assert_eq!(timezone.members[0].0, "Africa_Abidjan");
        assert!(
            timezone
                .members
                .iter()
                .any(|(name, _)| name == "Europe_Paris")
        );
    }

    #[test]
    fn encodes_timezone_punctuation_without_collisions() {
        let timezone = time_zone_enum();
        let mut names: Vec<_> = timezone.members.iter().map(|(name, _)| name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), timezone.members.len());
        assert!(
            timezone
                .members
                .iter()
                .any(|(name, _)| name == "Etc_GMT_Plus_1")
        );
        assert!(
            timezone
                .members
                .iter()
                .any(|(name, _)| name == "Etc_GMT_Minus_1")
        );
    }

    #[test]
    fn exports_dynamic_timezone_parsing_and_its_error_type() {
        let builtins = temporal_builtins();
        assert!(builtins.contains_key("TimeZone"));
        assert!(builtins.contains_key("TemporalError"));
        assert!(builtins.contains_key("ZonedDateTime"));
        let Type::Function { ret, .. } = &builtins["zonedDateTimeFrom"].ty else {
            panic!("zonedDateTimeFrom must export a function");
        };
        assert_eq!(
            ret.as_ref(),
            &Type::Result {
                success: Box::new(Type::Named("ZonedDateTime".to_string())),
                error: Box::new(Type::Named("TemporalError".to_string())),
            }
        );
        assert!(builtins["zonedDateTimeFrom"].throws.is_empty());
    }
}
