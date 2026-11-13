use crate::ast::*;
use crate::collections::HashSet;
use crate::span::Span;

pub(super) fn parse_db_model_decorator(
    decorator: ValidatorDecorator,
    legacy_timestamps: bool,
    fields: Vec<DbModelFieldMetadata>,
    relations: Vec<DbModelRelationMetadata>,
) -> Result<DbModelMetadata, String> {
    let (argument, options) = match decorator.args.as_slice() {
        [argument] => (argument, None),
        [argument, options] => (argument, Some(options)),
        _ => {
            return Err(
                "`@model` requires a string literal table name and optional options".to_string(),
            );
        }
    };
    let ExprKind::StrLit(table) = argument.kind() else {
        return Err("`@model` table name must be a string literal".to_string());
    };
    if table.is_empty() || table.chars().any(char::is_control) {
        return Err(
            "`@model` table name must be nonempty and contain no control characters".to_string(),
        );
    }
    let mut timestamps = true;
    let mut snake_case = false;
    if let Some(options) = options {
        let ExprKind::ObjectLit(options) = options.kind() else {
            return Err("`@model` options must be an object literal".to_string());
        };
        let mut seen = HashSet::default();
        for (name, value) in options {
            if !seen.insert(name.as_str()) {
                return Err(format!(
                    "`@model` option `{name}` is specified more than once"
                ));
            }
            match (name.as_str(), value.kind()) {
                ("timestamps", ExprKind::BoolLit(value)) => timestamps = *value,
                ("snakeCase", ExprKind::BoolLit(value)) => snake_case = *value,
                ("timestamps" | "snakeCase", _) => {
                    return Err(format!(
                        "`@model` option `{name}` must be a boolean literal"
                    ));
                }
                _ => return Err(format!("unknown `@model` option `{name}`")),
            }
        }
    }
    if legacy_timestamps && !timestamps {
        return Err("`@timestamps` cannot be combined with `{ timestamps: false }`".to_string());
    }
    if snake_case && !timestamps {
        return Err("`snakeCase` requires automatic Model timestamps".to_string());
    }
    Ok(DbModelMetadata {
        table: table.clone(),
        span: decorator.span,
        timestamps,
        snake_case,
        fields,
        relations,
    })
}

pub(super) fn parse_db_model_field_decorators(
    decorators: Vec<ValidatorDecorator>,
    field: &str,
    field_type: &Type,
    span: Span,
) -> Result<
    (
        Vec<ValidatorDecorator>,
        DbModelFieldMetadata,
        Option<DbModelRelationMetadata>,
    ),
    String,
> {
    let mut remaining = Vec::new();
    let mut column = field.to_string();
    let mut has_column = false;
    let mut id = false;
    let mut id_strategy = None;
    let mut unique = false;
    let mut default_value = None;
    let mut foreign_key = None;
    let mut relation = None;
    for decorator in decorators {
        match decorator.name.as_str() {
            "id" => {
                if id {
                    return Err(format!(
                        "field `{field}` has more than one `@id` decoration"
                    ));
                }
                id = true;
                match decorator.args.as_slice() {
                    [] => {}
                    [options] => {
                        let ExprKind::ObjectLit(options) = options.kind() else {
                            return Err("`@id` options must be an object literal".to_string());
                        };
                        let mut seen = HashSet::default();
                        for (name, value) in options {
                            if !seen.insert(name.as_str()) {
                                return Err(format!("`@id` option `{name}` is specified more than once"));
                            }
                            if name != "strategy" {
                                return Err(format!("unknown `@id` option `{name}`"));
                            }
                            let ExprKind::StrLit(strategy) = value.kind() else {
                                return Err("`@id` option `strategy` must be a string literal".to_string());
                            };
                            id_strategy = Some(strategy.clone());
                        }
                        if id_strategy.is_none() {
                            return Err("`@id` options require `strategy`".to_string());
                        }
                    }
                    _ => return Err("`@id` accepts no arguments or one options object: `@id({ strategy: \"...\" })`".to_string()),
                }
            }
            "unique" if decorator.args.is_empty() => {
                if unique {
                    return Err(format!(
                        "field `{field}` has more than one `@unique` decoration"
                    ));
                }
                unique = true;
            }
            "column" => {
                let [argument] = decorator.args.as_slice() else {
                    return Err("`@column` requires one string literal column name".to_string());
                };
                let ExprKind::StrLit(name) = argument.kind() else {
                    return Err("`@column` name must be a string literal".to_string());
                };
                if name.is_empty() || name.chars().any(char::is_control) || has_column {
                    return Err(format!(
                        "field `{field}` has an invalid or repeated `@column` mapping"
                    ));
                }
                column = name.clone();
                has_column = true;
            }
            "default" => {
                if default_value.is_some() {
                    return Err(format!(
                        "field `{field}` has more than one `@default` decoration"
                    ));
                }
                let [value] = decorator.args.as_slice() else {
                    return Err(
                        "`@default` requires exactly one compatible literal value".to_string()
                    );
                };
                if !matches!(
                    value.kind(),
                    ExprKind::StrLit(_)
                        | ExprKind::IntLit(_)
                        | ExprKind::UIntLit(_)
                        | ExprKind::FloatLit(_)
                        | ExprKind::BoolLit(_)
                ) {
                    return Err(
                        "`@default` requires a string, numeric, or boolean literal".to_string()
                    );
                }
                default_value = Some(value.clone());
            }
            "foreignKey" => {
                if foreign_key.is_some() {
                    return Err(format!(
                        "field `{field}` has more than one `@foreignKey` decoration"
                    ));
                }
                let [table, column] = decorator.args.as_slice() else {
                    return Err("`@foreignKey` requires a referenced table and column".to_string());
                };
                let (ExprKind::StrLit(table), ExprKind::StrLit(column)) =
                    (table.kind(), column.kind())
                else {
                    return Err(
                        "`@foreignKey` table and column must be string literals".to_string()
                    );
                };
                if table.is_empty()
                    || column.is_empty()
                    || table.chars().any(char::is_control)
                    || column.chars().any(char::is_control)
                {
                    return Err("`@foreignKey` table and column must be nonempty names".to_string());
                }
                foreign_key = Some((table.clone(), column.clone()));
            }
            "belongsTo" | "hasOne" | "hasMany" | "manyToMany" => {
                if relation.is_some() {
                    return Err(format!(
                        "field `{field}` has more than one relationship metadata decorator"
                    ));
                }
                let (foreign_key_field, pivot) = if decorator.name == "manyToMany" {
                    let [table, source_column, target_column] = decorator.args.as_slice() else {
                        return Err("`@manyToMany` requires a pivot table and its source and target foreign-key columns".to_string());
                    };
                    let (
                        ExprKind::StrLit(table),
                        ExprKind::StrLit(source_column),
                        ExprKind::StrLit(target_column),
                    ) = (table.kind(), source_column.kind(), target_column.kind())
                    else {
                        return Err("`@manyToMany` pivot table and foreign-key columns must be string literals".to_string());
                    };
                    if [table, source_column, target_column]
                        .iter()
                        .any(|value| value.is_empty() || value.chars().any(char::is_control))
                    {
                        return Err("`@manyToMany` pivot table and foreign-key columns must be nonempty names".to_string());
                    }
                    (
                        String::new(),
                        Some((table.clone(), source_column.clone(), target_column.clone())),
                    )
                } else {
                    let [foreign_key_field] = decorator.args.as_slice() else {
                        return Err(format!(
                            "`@{}` requires one foreign-key field name",
                            decorator.name
                        ));
                    };
                    let ExprKind::StrLit(foreign_key_field) = foreign_key_field.kind() else {
                        return Err(format!(
                            "`@{}` foreign-key field name must be a string literal",
                            decorator.name
                        ));
                    };
                    if foreign_key_field.is_empty()
                        || foreign_key_field.chars().any(char::is_control)
                    {
                        return Err(format!(
                            "`@{}` foreign-key field name must be nonempty",
                            decorator.name
                        ));
                    }
                    (foreign_key_field.clone(), None)
                };
                let target_model = match (decorator.name.as_str(), field_type) {
                    ("belongsTo" | "hasOne", Type::Optional(inner)) => match inner.as_ref() {
                        Type::Named(name) => name.clone(),
                        _ => return Err(format!("`@{}` relation field must have an optional named model type", decorator.name)),
                    },
                    ("hasMany", Type::Optional(inner)) => match inner.as_ref() {
                        Type::Array(element) => match element.as_ref() {
                            Type::Named(name) => name.clone(),
                            _ => return Err("`@hasMany` relation field must have an optional array of a named model type".to_string()),
                        },
                        _ => return Err("`@hasMany` relation field must have an optional array of a named model type".to_string()),
                    },
                    ("manyToMany", Type::Optional(inner)) => match inner.as_ref() {
                        Type::Array(element) => match element.as_ref() {
                            Type::Named(name) => name.clone(),
                            _ => return Err("`@manyToMany` relation field must have an optional array of a named model type".to_string()),
                        },
                        _ => return Err("`@manyToMany` relation field must have an optional array of a named model type".to_string()),
                    },
                    ("belongsTo" | "hasOne", _) => {
                        return Err(format!("`@{}` relation field must use an optional model type such as `Team?`", decorator.name));
                    }
                    ("hasMany", _) => {
                        return Err("`@hasMany` relation field must use an optional array type such as `User[]?`".to_string());
                    }
                    ("manyToMany", _) => {
                        return Err("`@manyToMany` relation field must use an optional array type such as `Role[]?`".to_string());
                    }
                    _ => unreachable!(),
                };
                relation = Some((
                    decorator.name.clone(),
                    foreign_key_field.clone(),
                    target_model,
                    pivot,
                ));
            }
            _ => remaining.push(decorator),
        }
    }
    let relation =
        relation.map(
            |(name, foreign_key_field, target_model, pivot)| DbModelRelationMetadata {
                field: field.to_string(),
                target_model,
                foreign_key_field,
                pivot_table: pivot.as_ref().map(|(table, _, _)| table.clone()),
                pivot_source_column: pivot.as_ref().map(|(_, source, _)| source.clone()),
                pivot_target_column: pivot.map(|(_, _, target)| target),
                kind: match name.as_str() {
                    "belongsTo" => DbModelRelationKind::BelongsTo,
                    "hasOne" => DbModelRelationKind::HasOne,
                    "hasMany" => DbModelRelationKind::HasMany,
                    "manyToMany" => DbModelRelationKind::ManyToMany,
                    _ => unreachable!(),
                },
                span,
            },
        );
    if relation.is_some()
        && (has_column || id || unique || default_value.is_some() || foreign_key.is_some())
    {
        return Err(format!(
            "relationship field `{field}` cannot also declare database column metadata"
        ));
    }
    Ok((
        remaining,
        DbModelFieldMetadata {
            field: field.to_string(),
            column,
            ty: field_type.clone(),
            id,
            id_strategy,
            unique,
            default_value,
            foreign_key,
            span,
        },
        relation,
    ))
}
