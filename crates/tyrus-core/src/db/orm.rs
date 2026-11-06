use super::MAX_BOUND_PARAMETERS as SQLITE_SAFE_PARAMETER_LIMIT;
use crate::ast::{DbModelMetadata, DbModelRelationKind, Expr, ExprKind, Field, Type};
use crate::collections::HashMap;

#[derive(Clone, Debug)]
pub struct OrmSelect {
    pub sql: String,
    pub parameters: Vec<Expr>,
}

pub const MAX_BATCH_ROWS: usize = 500;
pub const MAX_RELATION_ROWS: u64 = 500;

pub const JSON_RELATION_ALIAS_PREFIX: &str = "__tyrus_json__";
pub const INSTANT_ALIAS_PREFIX: &str = "__tyrus_instant__";
pub const UNSIGNED_ALIAS_PREFIX: &str = "__tyrus_u64__";
const POSTGRES_UUID_PARAMETER: &str = "?/*__tyrus_uuid_parameter__*/";

fn model_field_uses_uuid(field: &crate::ast::DbModelFieldMetadata) -> bool {
    matches!(&field.ty, Type::Named(name) if name == "Uuid" || name == "Uuid7")
}

fn model_parameter_placeholder(uuid: bool) -> &'static str {
    if uuid { POSTGRES_UUID_PARAMETER } else { "?" }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DbRelationInclude {
    pub field: String,
    pub projection: Option<Vec<String>>,
}

pub fn project_model_result_type(
    model_name: &str,
    models: &HashMap<String, DbModelMetadata>,
    interfaces: &HashMap<String, Vec<Field>>,
    includes: &[DbRelationInclude],
) -> Result<Type, String> {
    let model = models
        .get(model_name)
        .ok_or_else(|| format!("unknown database Model `{model_name}`"))?;
    let fields = interfaces
        .get(model_name)
        .ok_or_else(|| format!("Model `{model_name}` has no interface fields"))?;
    let mut projected = fields.clone();
    for include in includes {
        let relation = model
            .relations
            .iter()
            .find(|relation| relation.field == include.field)
            .ok_or_else(|| format!("unknown relationship `{}`", include.field))?;
        let Some(selection) = &include.projection else {
            continue;
        };
        let target_fields = interfaces.get(&relation.target_model).ok_or_else(|| {
            format!(
                "related Model `{}` has no interface fields",
                relation.target_model
            )
        })?;
        let selected = target_fields
            .iter()
            .filter(|field| selection.iter().any(|name| name == &field.name))
            .cloned()
            .collect::<Vec<_>>();
        if selected.len() != selection.len() {
            return Err(format!(
                "projection for relationship `{}` contains an unknown field",
                include.field
            ));
        }
        let target = Type::Object(selected);
        let relation_type = match relation.kind {
            DbModelRelationKind::BelongsTo | DbModelRelationKind::HasOne => {
                Type::Optional(Box::new(target))
            }
            DbModelRelationKind::HasMany | DbModelRelationKind::ManyToMany => {
                Type::Optional(Box::new(Type::Array(Box::new(target))))
            }
        };
        let field = projected
            .iter_mut()
            .find(|field| field.name == include.field)
            .ok_or_else(|| format!("relationship `{}` is not an interface field", include.field))?;
        field.ty = relation_type;
    }
    Ok(Type::Object(projected))
}

pub fn find_many_with_relations_query(
    model_name: &str,
    models: &HashMap<String, DbModelMetadata>,
    filter: &Expr,
    includes: &[DbRelationInclude],
    paginate: bool,
    cursor: bool,
    relation_limit: Option<u64>,
) -> Result<OrmSelect, String> {
    let model = models
        .get(model_name)
        .ok_or_else(|| format!("unknown database Model `{model_name}`"))?;
    if includes.is_empty() {
        let mut query = find_many_query(model, filter)?;
        if cursor {
            let primary = model
                .fields
                .iter()
                .find(|field| field.id)
                .ok_or_else(|| "cursor pagination requires a model `@id` field".to_string())?;
            append_cursor_filter(&mut query.sql, &primary.column);
        }
        if paginate || cursor {
            append_model_ordering(&mut query.sql, model)?;
        }
        return Ok(query);
    }

    let mut selected = model
        .fields
        .iter()
        .map(|field| model_projection("__tyrus_root", field))
        .collect::<Vec<_>>();
    let mut joins = Vec::new();
    let mut parameters = Vec::new();
    let mut predicates = Vec::new();
    append_filter_qualified(
        model,
        filter,
        "__tyrus_root",
        &mut predicates,
        &mut parameters,
    )?;
    let mut seen = std::collections::HashSet::new();

    for (index, include) in includes.iter().enumerate() {
        let field_name = include.field.as_str();
        if !seen.insert(field_name) {
            return Err(format!(
                "relation `{field_name}` is included more than once"
            ));
        }
        let relation = model
            .relations
            .iter()
            .find(|relation| relation.field == *field_name)
            .ok_or_else(|| {
                format!("unknown relationship field `{field_name}` on Model `{model_name}`")
            })?;
        let related = models
            .get(&relation.target_model)
            .ok_or_else(|| format!("relationship `{field_name}` targets an unavailable Model"))?;
        let collection_relation = matches!(
            relation.kind,
            DbModelRelationKind::HasMany | DbModelRelationKind::ManyToMany
        );
        if collection_relation && relation_limit.is_none() {
            return Err(format!(
                "relationship `{field_name}` requires an explicit row limit"
            ));
        }
        if let Some(limit) = relation_limit
            && !(1..=MAX_RELATION_ROWS).contains(&limit)
        {
            return Err(format!(
                "relationship row limit must be between 1 and {MAX_RELATION_ROWS}"
            ));
        }
        let alias = format!("__tyrus_relation_{index}");
        if relation.kind == DbModelRelationKind::ManyToMany {
            let source_key = model.fields.iter().find(|field| field.id).ok_or_else(|| {
                format!("source Model `{model_name}` requires an `@id` field for pivot relations")
            })?;
            let target_key = related
                .fields
                .iter()
                .find(|field| field.id)
                .ok_or_else(|| {
                    format!(
                        "related Model `{}` requires an `@id` field",
                        relation.target_model
                    )
                })?;
            let pivot_table = relation
                .pivot_table
                .as_deref()
                .ok_or_else(|| format!("relationship `{field_name}` has no pivot table"))?;
            let pivot_source_column = relation
                .pivot_source_column
                .as_deref()
                .ok_or_else(|| format!("relationship `{field_name}` has no source pivot key"))?;
            let pivot_target_column = relation
                .pivot_target_column
                .as_deref()
                .ok_or_else(|| format!("relationship `{field_name}` has no target pivot key"))?;
            let pivot_alias = format!("__tyrus_pivot_{index}");
            let row_limit = relation_limit
                .map(|limit| format!(" LIMIT {limit}"))
                .unwrap_or_default();
            selected.push(format!(
                "(SELECT json_group_array(json(__tyrus_json)) FROM (SELECT {} AS __tyrus_json FROM {} AS {} INNER JOIN {} AS {} ON {} = {} WHERE {} = {} ORDER BY {}{})) AS {}",
                json_object_sql(related, &alias, include.projection.as_deref()),
                quote_identifier(&related.table),
                quote_identifier(&alias),
                quote_identifier(pivot_table),
                quote_identifier(&pivot_alias),
                qualified_column(&alias, &target_key.column),
                qualified_column(&pivot_alias, pivot_target_column),
                qualified_column(&pivot_alias, pivot_source_column),
                qualified_column("__tyrus_root", &source_key.column),
                qualified_column(&alias, &target_key.column),
                row_limit,
                quote_identifier(&format!("{JSON_RELATION_ALIAS_PREFIX}{field_name}")),
            ));
            continue;
        }
        let (root_column, related_column) = relation_columns(model, related, relation)?;
        let object = json_object_sql(related, &alias, include.projection.as_deref());
        match relation.kind {
            DbModelRelationKind::BelongsTo | DbModelRelationKind::HasOne => {
                let related_id = related
                    .fields
                    .iter()
                    .find(|field| field.id)
                    .ok_or_else(|| {
                        format!(
                            "related Model `{}` requires an `@id` field",
                            relation.target_model
                        )
                    })?;
                joins.push(format!(
                    "LEFT JOIN {} AS {} ON {} = {}",
                    quote_identifier(&related.table),
                    quote_identifier(&alias),
                    qualified_column("__tyrus_root", root_column),
                    qualified_column(&alias, related_column)
                ));
                selected.push(format!(
                    "CASE WHEN {} IS NULL THEN NULL ELSE {object} END AS {}",
                    qualified_column(&alias, &related_id.column),
                    quote_identifier(&format!("{JSON_RELATION_ALIAS_PREFIX}{field_name}"))
                ));
            }
            DbModelRelationKind::HasMany => {
                let related_id = related
                    .fields
                    .iter()
                    .find(|field| field.id)
                    .ok_or_else(|| {
                        format!(
                            "related Model `{}` requires an `@id` field",
                            relation.target_model
                        )
                    })?;
                let row_limit = relation_limit
                    .map(|limit| format!(" LIMIT {limit}"))
                    .unwrap_or_default();
                selected.push(format!(
                    "(SELECT json_group_array(json(__tyrus_json)) FROM (SELECT {object} AS __tyrus_json FROM {} AS {} WHERE {} = {} ORDER BY {}{})) AS {}",
                    quote_identifier(&related.table),
                    quote_identifier(&alias),
                    qualified_column(&alias, related_column),
                    qualified_column("__tyrus_root", root_column),
                    qualified_column(&alias, &related_id.column),
                    row_limit,
                    quote_identifier(&format!("{JSON_RELATION_ALIAS_PREFIX}{field_name}"))
                ));
            }
            DbModelRelationKind::ManyToMany => unreachable!("handled before direct relations"),
        }
    }
    let mut sql = format!(
        "SELECT {} FROM {} AS {}",
        selected.join(", "),
        quote_identifier(&model.table),
        quote_identifier("__tyrus_root")
    );
    if !joins.is_empty() {
        sql.push(' ');
        sql.push_str(&joins.join(" "));
    }
    if !predicates.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&predicates.join(" AND "));
    }
    if cursor {
        let primary = model
            .fields
            .iter()
            .find(|field| field.id)
            .ok_or_else(|| "cursor pagination requires a model `@id` field".to_string())?;
        append_cursor_filter(&mut sql, &primary.column);
    }
    if paginate || cursor {
        append_model_ordering(&mut sql, model)?;
    }
    if parameters.len() > SQLITE_SAFE_PARAMETER_LIMIT {
        return Err(format!(
            "relationship query exceeds SQLite's safe limit of {SQLITE_SAFE_PARAMETER_LIMIT} parameters"
        ));
    }
    Ok(OrmSelect { sql, parameters })
}

fn relation_columns<'a>(
    model: &'a DbModelMetadata,
    related: &'a DbModelMetadata,
    relation: &'a crate::ast::DbModelRelationMetadata,
) -> Result<(&'a str, &'a str), String> {
    match relation.kind {
        DbModelRelationKind::BelongsTo => {
            let local = model
                .fields
                .iter()
                .find(|field| field.field == relation.foreign_key_field)
                .ok_or_else(|| {
                    format!("relationship `{}` has no local foreign key", relation.field)
                })?;
            let (_, referenced_column) = local
                .foreign_key
                .as_ref()
                .ok_or_else(|| format!("relationship `{}` has no `@foreignKey`", relation.field))?;
            let target = related
                .fields
                .iter()
                .find(|field| field.column.eq_ignore_ascii_case(referenced_column))
                .ok_or_else(|| {
                    format!(
                        "relationship `{}` references an unknown target column",
                        relation.field
                    )
                })?;
            Ok((&local.column, &target.column))
        }
        DbModelRelationKind::HasOne | DbModelRelationKind::HasMany => {
            let remote = related
                .fields
                .iter()
                .find(|field| field.field == relation.foreign_key_field)
                .ok_or_else(|| {
                    format!(
                        "relationship `{}` has no related foreign key",
                        relation.field
                    )
                })?;
            let (_, referenced_column) = remote.foreign_key.as_ref().ok_or_else(|| {
                format!(
                    "relationship `{}` has no related `@foreignKey`",
                    relation.field
                )
            })?;
            let local = model
                .fields
                .iter()
                .find(|field| field.column.eq_ignore_ascii_case(referenced_column))
                .ok_or_else(|| {
                    format!(
                        "relationship `{}` references an unknown local column",
                        relation.field
                    )
                })?;
            Ok((&local.column, &remote.column))
        }
        DbModelRelationKind::ManyToMany => {
            Err("many-to-many relationships use pivot columns".to_string())
        }
    }
}

fn json_object_sql(model: &DbModelMetadata, alias: &str, projection: Option<&[String]>) -> String {
    format!(
        "json_object({})",
        model
            .fields
            .iter()
            .filter(|field| {
                projection
                    .is_none_or(|projection| projection.iter().any(|name| name == &field.field))
            })
            .flat_map(|field| [
                format!(
                    "'{}'",
                    if is_instant_type(&field.ty) {
                        format!("{INSTANT_ALIAS_PREFIX}{}", field.field).replace('\'', "''")
                    } else {
                        field.field.replace('\'', "''")
                    }
                ),
                qualified_column(alias, &field.column),
            ])
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn model_projection(alias: &str, field: &crate::ast::DbModelFieldMetadata) -> String {
    let result_name = if is_u64_type(&field.ty) {
        format!("{UNSIGNED_ALIAS_PREFIX}{}", field.field)
    } else if is_instant_type(&field.ty) {
        format!("{INSTANT_ALIAS_PREFIX}{}", field.field)
    } else {
        field.field.clone()
    };
    format!(
        "{} AS {}",
        qualified_column(alias, &field.column),
        quote_identifier(&result_name)
    )
}

pub fn postgresql_instant_projection_sql(sql: &str, model: &DbModelMetadata) -> String {
    let mut sql = sql.to_string();
    for field in model
        .fields
        .iter()
        .filter(|field| is_instant_type(&field.ty))
    {
        let alias = format!("{INSTANT_ALIAS_PREFIX}{}", field.field);
        for expression in [
            qualified_column("__tyrus_root", &field.column),
            quote_identifier(&field.column),
        ] {
            let projection = format!("{expression} AS {}", quote_identifier(&alias));
            let replacement = format!(
                "CAST(CAST({expression} AS TIMESTAMP) AS TEXT) AS {}",
                quote_identifier(&alias),
            );
            sql = sql.replace(&projection, &replacement);
        }
    }
    sql
}

pub fn mysql_instant_projection_sql(sql: &str, model: &DbModelMetadata) -> String {
    let mut sql = sql.to_string();
    for field in model
        .fields
        .iter()
        .filter(|field| is_instant_type(&field.ty))
    {
        let alias = format!("{INSTANT_ALIAS_PREFIX}{}", field.field);
        for expression in [
            qualified_column("__tyrus_root", &field.column),
            quote_identifier(&field.column),
        ] {
            let projection = format!("{expression} AS {}", quote_identifier(&alias));
            let replacement =
                format!("CAST({expression} AS CHAR) AS {}", quote_identifier(&alias),);
            sql = sql.replace(&projection, &replacement);
        }
    }
    sql
}

fn is_u64_type(ty: &Type) -> bool {
    match ty {
        Type::U64 => true,
        Type::Optional(inner) | Type::Readonly(inner) => is_u64_type(inner),
        _ => false,
    }
}

fn is_instant_type(ty: &Type) -> bool {
    match ty {
        Type::Named(name) => name == "Instant",
        Type::Optional(inner) | Type::Readonly(inner) => is_instant_type(inner),
        _ => false,
    }
}

fn append_model_ordering(sql: &mut String, model: &DbModelMetadata) -> Result<(), String> {
    let primary =
        model.fields.iter().find(|field| field.id).ok_or_else(|| {
            "paginated Models require an `@id` field for stable ordering".to_string()
        })?;
    sql.push_str(&format!(
        " ORDER BY {} ASC",
        qualified_column("__tyrus_root", &primary.column)
    ));
    Ok(())
}

fn append_cursor_filter(sql: &mut String, id_column: &str) {
    let predicate = format!(
        "(? = 0 OR {} > ?)",
        qualified_column("__tyrus_root", id_column)
    );
    sql.push_str(if sql.contains(" WHERE ") {
        " AND "
    } else {
        " WHERE "
    });
    sql.push_str(&predicate);
}

fn qualified_column(alias: &str, column: &str) -> String {
    format!("{}.{}", quote_identifier(alias), quote_identifier(column))
}

fn append_filter_qualified(
    model: &DbModelMetadata,
    expression: &Expr,
    alias: &str,
    output: &mut Vec<String>,
    parameters: &mut Vec<Expr>,
) -> Result<(), String> {
    let ExprKind::ObjectLit(fields) = expression.kind() else {
        return Err("database filters must be object literals".to_string());
    };
    for (name, value) in fields {
        if name == "or" {
            let ExprKind::ArrayLit(alternatives) = value.kind() else {
                return Err("the `or` filter must be an array of filter objects".to_string());
            };
            if alternatives.is_empty() {
                return Err("the `or` filter must contain at least one alternative".to_string());
            }
            let mut groups = Vec::new();
            for alternative in alternatives {
                let mut predicates = Vec::new();
                append_filter_qualified(model, alternative, alias, &mut predicates, parameters)?;
                if predicates.is_empty() {
                    return Err("each `or` alternative must contain a filter".to_string());
                }
                groups.push(format!("({})", predicates.join(" AND ")));
            }
            output.push(format!("({})", groups.join(" OR ")));
            continue;
        }
        let metadata = model
            .fields
            .iter()
            .find(|field| field.field == *name)
            .ok_or_else(|| format!("unknown model filter field `{name}`"))?;
        append_qualified_field_filter(
            &qualified_column(alias, &metadata.column),
            value,
            output,
            parameters,
            model_field_uses_uuid(metadata),
        )?;
    }
    Ok(())
}

fn append_qualified_field_filter(
    column: &str,
    value: &Expr,
    output: &mut Vec<String>,
    parameters: &mut Vec<Expr>,
    uuid: bool,
) -> Result<(), String> {
    let placeholder = model_parameter_placeholder(uuid);
    if let ExprKind::ObjectLit(operators) = value.kind() {
        if operators.is_empty() {
            return Err("comparison filters must contain at least one operator".to_string());
        }
        for (operator, operand) in operators {
            match operator.as_str() {
                "eq" | "ne" | "gt" | "gte" | "lt" | "lte" => {
                    let op = match operator.as_str() {
                        "eq" => "=",
                        "ne" => "<>",
                        "gt" => ">",
                        "gte" => ">=",
                        "lt" => "<",
                        "lte" => "<=",
                        _ => unreachable!(),
                    };
                    output.push(format!("{column} {op} {placeholder}"));
                    parameters.push(operand.clone());
                }
                "in" => {
                    let ExprKind::ArrayLit(values) = operand.kind() else {
                        return Err(
                            "the `in` filter operator requires an array literal".to_string()
                        );
                    };
                    if values.is_empty() {
                        return Err(
                            "the `in` filter operator requires at least one value".to_string()
                        );
                    }
                    output.push(format!(
                        "{column} IN ({})",
                        vec![placeholder; values.len()].join(", ")
                    ));
                    parameters.extend(values.iter().cloned());
                }
                "like" => {
                    output.push(format!("{column} LIKE {placeholder}"));
                    parameters.push(operand.clone());
                }
                _ => return Err(format!("unsupported filter operator `{operator}`")),
            }
        }
    } else {
        output.push(format!("{column} = {placeholder}"));
        parameters.push(value.clone());
    }
    Ok(())
}

pub fn mutation_query(
    operation: &str,
    model: &DbModelMetadata,
    arguments: &[Expr],
) -> Result<OrmSelect, String> {
    match operation {
        "insertOne" => {
            if arguments.len() != 1 {
                return Err("`insertOne` expects one model object".to_string());
            }
            let Some(value) = arguments.first() else {
                return Err("`insertOne` requires one model object".to_string());
            };
            insert_many_query(model, std::slice::from_ref(value))
        }
        "insertMany" => {
            if arguments.len() != 1 {
                return Err("`insertMany` requires an array of model objects".to_string());
            }
            let ExprKind::ArrayLit(rows) = arguments[0].kind() else {
                return Err("`insertMany` requires an array of model objects".to_string());
            };
            insert_many_query(model, rows)
        }
        "updateOne" | "updateMany" => {
            if arguments.len() != 2 {
                return Err(format!(
                    "`{operation}` requires a filter and changes object"
                ));
            }
            if operation == "updateOne" && !filters_unique_model_field(model, &arguments[0]) {
                return Err(
                    "`updateOne` requires an equality filter on an `@id` or `@unique` field"
                        .to_string(),
                );
            }
            update_query(model, &arguments[0], &arguments[1])
        }
        "deleteOne" | "deleteMany" => {
            if arguments.len() != 1 {
                return Err(format!(
                    "`{operation}` requires one non-empty filter object"
                ));
            }
            if operation == "deleteOne" && !filters_unique_model_field(model, &arguments[0]) {
                return Err(
                    "`deleteOne` requires an equality filter on an `@id` or `@unique` field"
                        .to_string(),
                );
            }
            delete_query(model, &arguments[0])
        }
        _ => Err(format!("unsupported model operation `{operation}`")),
    }
}

pub fn mutation_returning_clause(model: &DbModelMetadata) -> String {
    model
        .fields
        .iter()
        .map(|field| {
            let alias = if is_u64_type(&field.ty) {
                format!("{UNSIGNED_ALIAS_PREFIX}{}", field.field)
            } else if is_instant_type(&field.ty) {
                format!("{INSTANT_ALIAS_PREFIX}{}", field.field)
            } else {
                field.field.clone()
            };
            format!(
                "{} AS {}",
                quote_identifier(&field.column),
                quote_identifier(&alias)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn filters_unique_model_field(model: &DbModelMetadata, filter: &Expr) -> bool {
    let ExprKind::ObjectLit(fields) = filter.kind() else {
        return false;
    };
    fields.iter().any(|(name, value)| {
        let Some(field) = model.fields.iter().find(|field| field.field == *name) else {
            return false;
        };
        if !(field.id || field.unique) {
            return false;
        }
        match value.kind() {
            ExprKind::ObjectLit(operators) => operators.len() == 1 && operators[0].0 == "eq",
            _ => true,
        }
    })
}

fn insert_many_query(model: &DbModelMetadata, rows: &[Expr]) -> Result<OrmSelect, String> {
    if rows.is_empty() || rows.len() > MAX_BATCH_ROWS {
        return Err(format!(
            "insert batches must contain between 1 and {MAX_BATCH_ROWS} rows"
        ));
    }
    let mut columns: Option<Vec<&str>> = None;
    let mut parameters = Vec::new();
    let mut value_groups = Vec::new();
    for row in rows {
        let ExprKind::ObjectLit(fields) = row.kind() else {
            return Err("insert values must be object literals".to_string());
        };
        if fields.is_empty() {
            return Err("insert values must contain at least one field".to_string());
        }
        let mut row_columns = Vec::with_capacity(fields.len());
        let mut row_values = Vec::with_capacity(fields.len());
        for (name, value) in fields {
            let field = model
                .fields
                .iter()
                .find(|field| field.field == *name)
                .ok_or_else(|| format!("unknown model insert field `{name}`"))?;
            row_columns.push(field.column.as_str());
            row_values.push(model_parameter_placeholder(model_field_uses_uuid(field)));
            parameters.push(value.clone());
        }
        if let Some(expected) = &columns {
            if *expected != row_columns {
                return Err(
                    "every insertMany row must provide the same fields in the same order"
                        .to_string(),
                );
            }
        } else {
            columns = Some(row_columns);
        }
        value_groups.push(format!("({})", row_values.join(", ")));
    }
    if parameters.len() > SQLITE_SAFE_PARAMETER_LIMIT {
        return Err(format!(
            "insert batch exceeds SQLite's safe limit of {SQLITE_SAFE_PARAMETER_LIMIT} parameters"
        ));
    }
    let columns = columns.ok_or_else(|| "insert values are required".to_string())?;
    Ok(OrmSelect {
        sql: format!(
            "INSERT INTO {} ({}) VALUES {}",
            quote_identifier(&model.table),
            columns
                .iter()
                .map(|column| quote_identifier(column))
                .collect::<Vec<_>>()
                .join(", "),
            value_groups.join(", ")
        ),
        parameters,
    })
}

fn update_query(
    model: &DbModelMetadata,
    filter: &Expr,
    changes: &Expr,
) -> Result<OrmSelect, String> {
    let ExprKind::ObjectLit(fields) = changes.kind() else {
        return Err("update changes must be an object literal".to_string());
    };
    if fields.is_empty() {
        return Err("update changes must contain at least one field".to_string());
    }
    let mut assignments = Vec::with_capacity(fields.len());
    let mut parameters = Vec::with_capacity(fields.len());
    for (name, value) in fields {
        let field = model
            .fields
            .iter()
            .find(|field| field.field == *name)
            .ok_or_else(|| format!("unknown model update field `{name}`"))?;
        if field.id {
            return Err(format!("model id field `{name}` cannot be updated"));
        }
        if model.timestamps && field.field == "updatedAt" {
            return Err("Model field `updatedAt` is maintained automatically".to_string());
        }
        if model.timestamps && field.field == "createdAt" {
            return Err("Model field `createdAt` cannot be changed".to_string());
        }
        assignments.push(format!(
            "{} = {}",
            quote_identifier(&field.column),
            model_parameter_placeholder(model_field_uses_uuid(field))
        ));
        parameters.push(value.clone());
    }
    if model.timestamps {
        let updated = model
            .fields
            .iter()
            .find(|field| field.field == "updatedAt")
            .ok_or_else(|| "timestamp-enabled Model has no `updatedAt` field".to_string())?;
        assignments.push(format!(
            "{} = CURRENT_TIMESTAMP",
            quote_identifier(&updated.column)
        ));
    }
    let predicates = append_required_filter(model, filter, &mut parameters)?;
    Ok(OrmSelect {
        sql: format!(
            "UPDATE {} SET {} WHERE {}",
            quote_identifier(&model.table),
            assignments.join(", "),
            predicates.join(" AND ")
        ),
        parameters,
    })
}

fn delete_query(model: &DbModelMetadata, filter: &Expr) -> Result<OrmSelect, String> {
    let mut parameters = Vec::new();
    let predicates = append_required_filter(model, filter, &mut parameters)?;
    Ok(OrmSelect {
        sql: format!(
            "DELETE FROM {} WHERE {}",
            quote_identifier(&model.table),
            predicates.join(" AND ")
        ),
        parameters,
    })
}

fn append_required_filter(
    model: &DbModelMetadata,
    filter: &Expr,
    parameters: &mut Vec<Expr>,
) -> Result<Vec<String>, String> {
    if !matches!(filter.kind(), ExprKind::ObjectLit(fields) if !fields.is_empty()) {
        return Err("mutation filters must be non-empty object literals".to_string());
    }
    let mut predicates = Vec::new();
    append_filter(model, filter, &mut predicates, parameters)?;
    if predicates.is_empty() {
        return Err("mutation filters must match at least one model field".to_string());
    }
    if parameters.len() > SQLITE_SAFE_PARAMETER_LIMIT {
        return Err(format!(
            "mutation exceeds SQLite's safe limit of {SQLITE_SAFE_PARAMETER_LIMIT} parameters"
        ));
    }
    Ok(predicates)
}

pub fn find_many_query(model: &DbModelMetadata, filter: &Expr) -> Result<OrmSelect, String> {
    let mut parameters = Vec::new();
    let mut predicates = Vec::new();
    append_filter(model, filter, &mut predicates, &mut parameters)?;
    let where_clause = if predicates.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", predicates.join(" AND "))
    };
    Ok(OrmSelect {
        sql: format!(
            "SELECT {} FROM {} AS {}{}",
            model
                .fields
                .iter()
                .map(|field| model_projection("__tyrus_root", field))
                .collect::<Vec<_>>()
                .join(", "),
            quote_identifier(&model.table),
            quote_identifier("__tyrus_root"),
            where_clause
        ),
        parameters,
    })
}

pub fn find_many_page_query(model: &DbModelMetadata, filter: &Expr) -> Result<OrmSelect, String> {
    let primary_key =
        model.fields.iter().find(|field| field.id).ok_or_else(|| {
            "paginated models require an `@id` field for stable ordering".to_string()
        })?;
    let mut query = find_many_query(model, filter)?;
    query.sql.push_str(&format!(
        " ORDER BY {} ASC",
        quote_identifier(&primary_key.column)
    ));
    Ok(query)
}

pub fn find_many_cursor_query(model: &DbModelMetadata, filter: &Expr) -> Result<OrmSelect, String> {
    let primary_key = model.fields.iter().find(|field| field.id).ok_or_else(|| {
        "cursor pagination requires a model `@id` field for stable ordering".to_string()
    })?;
    let mut query = find_many_query(model, filter)?;
    let cursor_predicate = format!(
        "(? = 0 OR {} > {})",
        quote_identifier(&primary_key.column),
        model_parameter_placeholder(model_field_uses_uuid(primary_key))
    );
    if query.sql.contains(" WHERE ") {
        query.sql.push_str(" AND ");
        query.sql.push_str(&cursor_predicate);
    } else {
        query.sql.push_str(" WHERE ");
        query.sql.push_str(&cursor_predicate);
    }
    query.sql.push_str(&format!(
        " ORDER BY {} ASC",
        quote_identifier(&primary_key.column)
    ));
    Ok(query)
}

pub fn find_one_query(model: &DbModelMetadata, filter: &Expr) -> Result<OrmSelect, String> {
    let mut query = find_many_query(model, filter)?;
    query.sql.push_str(" LIMIT 1");
    Ok(query)
}

pub fn find_one_by_id_parameter_query(model: &DbModelMetadata) -> Result<OrmSelect, String> {
    let id = model
        .fields
        .iter()
        .find(|field| field.id)
        .ok_or_else(|| "Model mutation readback requires an `@id` field".to_string())?;
    let projection = model
        .fields
        .iter()
        .map(|field| model_projection("__tyrus_root", field))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(OrmSelect {
        sql: format!(
            "SELECT {projection} FROM {} AS {} WHERE {} = ? LIMIT 1",
            quote_identifier(&model.table),
            quote_identifier("__tyrus_root"),
            qualified_column("__tyrus_root", &id.column)
        ),
        parameters: Vec::new(),
    })
}

pub fn count_query(model: &DbModelMetadata, filter: &Expr) -> Result<OrmSelect, String> {
    let mut parameters = Vec::new();
    let mut predicates = Vec::new();
    append_filter(model, filter, &mut predicates, &mut parameters)?;
    if parameters.len() > SQLITE_SAFE_PARAMETER_LIMIT {
        return Err(format!(
            "count filter exceeds SQLite's safe limit of {SQLITE_SAFE_PARAMETER_LIMIT} parameters"
        ));
    }
    let where_clause = if predicates.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", predicates.join(" AND "))
    };
    Ok(OrmSelect {
        sql: format!(
            "SELECT COUNT(*) AS \"count\" FROM {}{}",
            quote_identifier(&model.table),
            where_clause
        ),
        parameters,
    })
}

fn append_filter(
    model: &DbModelMetadata,
    expression: &Expr,
    output: &mut Vec<String>,
    parameters: &mut Vec<Expr>,
) -> Result<(), String> {
    let ExprKind::ObjectLit(fields) = expression.kind() else {
        return Err("database filters must be object literals".to_string());
    };
    for (name, value) in fields {
        if name == "or" {
            let ExprKind::ArrayLit(alternatives) = value.kind() else {
                return Err("the `or` filter must be an array of filter objects".to_string());
            };
            if alternatives.is_empty() {
                return Err("the `or` filter must contain at least one alternative".to_string());
            }
            let mut groups = Vec::with_capacity(alternatives.len());
            for alternative in alternatives {
                let mut predicates = Vec::new();
                append_filter(model, alternative, &mut predicates, parameters)?;
                if predicates.is_empty() {
                    return Err("each `or` alternative must contain a filter".to_string());
                }
                groups.push(format!("({})", predicates.join(" AND ")));
            }
            output.push(format!("({})", groups.join(" OR ")));
            continue;
        }
        let metadata = model
            .fields
            .iter()
            .find(|field| field.field == *name)
            .ok_or_else(|| format!("unknown model filter field `{name}`"))?;
        append_field_filter(
            &metadata.column,
            value,
            output,
            parameters,
            model_field_uses_uuid(metadata),
        )?;
    }
    Ok(())
}

fn append_field_filter(
    column: &str,
    value: &Expr,
    output: &mut Vec<String>,
    parameters: &mut Vec<Expr>,
    uuid: bool,
) -> Result<(), String> {
    let quoted = quote_identifier(column);
    let placeholder = model_parameter_placeholder(uuid);
    if let ExprKind::ObjectLit(operators) = value.kind() {
        if operators.is_empty() {
            return Err("comparison filters must contain at least one operator".to_string());
        }
        for (operator, operand) in operators {
            match operator.as_str() {
                "eq" | "ne" | "gt" | "gte" | "lt" | "lte" => {
                    let sql_operator = match operator.as_str() {
                        "eq" => "=",
                        "ne" => "<>",
                        "gt" => ">",
                        "gte" => ">=",
                        "lt" => "<",
                        "lte" => "<=",
                        _ => unreachable!(),
                    };
                    output.push(format!("{quoted} {sql_operator} {placeholder}"));
                    parameters.push(operand.clone());
                }
                "in" => {
                    let ExprKind::ArrayLit(values) = operand.kind() else {
                        return Err("the `in` filter operator requires an array".to_string());
                    };
                    if values.is_empty() {
                        return Err(
                            "the `in` filter operator requires at least one value".to_string()
                        );
                    }
                    let alternatives = values
                        .iter()
                        .map(|value| {
                            parameters.push(value.clone());
                            format!("{quoted} = {placeholder}")
                        })
                        .collect::<Vec<_>>();
                    output.push(format!("({})", alternatives.join(" OR ")));
                }
                "like" => {
                    output.push(format!("{quoted} LIKE {placeholder}"));
                    parameters.push(operand.clone());
                }
                _ => return Err(format!("unsupported filter operator `{operator}`")),
            }
        }
    } else {
        output.push(format!("{quoted} = {placeholder}"));
        parameters.push(value.clone());
    }
    Ok(())
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod instant_projection_tests {
    use super::{mysql_instant_projection_sql, postgresql_instant_projection_sql};
    use crate::ast::{DbModelFieldMetadata, DbModelMetadata, Type};
    use crate::span::Span;

    fn model() -> DbModelMetadata {
        DbModelMetadata {
            table: "users".to_string(),
            span: Span::DUMMY,
            timestamps: true,
            snake_case: false,
            fields: vec![DbModelFieldMetadata {
                field: "createdAt".to_string(),
                column: "createdAt".to_string(),
                ty: Type::Named("Instant".to_string()),
                id: false,
                id_strategy: None,
                unique: false,
                default_value: None,
                foreign_key: None,
                span: Span::DUMMY,
            }],
            relations: Vec::new(),
        }
    }

    #[test]
    fn casts_generated_postgresql_instant_read_and_returning_projections() {
        let source = "SELECT \"__tyrus_root\".\"createdAt\" AS \"__tyrus_instant__createdAt\" FROM \"users\" RETURNING \"createdAt\" AS \"__tyrus_instant__createdAt\"";
        let sql = postgresql_instant_projection_sql(source, &model());

        assert_eq!(
            sql,
            "SELECT CAST(CAST(\"__tyrus_root\".\"createdAt\" AS TIMESTAMP) AS TEXT) AS \"__tyrus_instant__createdAt\" FROM \"users\" RETURNING CAST(CAST(\"createdAt\" AS TIMESTAMP) AS TEXT) AS \"__tyrus_instant__createdAt\""
        );
    }

    #[test]
    fn leaves_non_instant_and_unmarked_projections_unchanged() {
        let source = "SELECT \"__tyrus_root\".\"createdAt\" AS \"createdAt\" FROM \"users\"";
        assert_eq!(postgresql_instant_projection_sql(source, &model()), source);
    }

    #[test]
    fn casts_generated_mysql_instant_readback_projections_to_text() {
        let source = "SELECT \"__tyrus_root\".\"createdAt\" AS \"__tyrus_instant__createdAt\" FROM \"users\"";
        assert_eq!(
            mysql_instant_projection_sql(source, &model()),
            "SELECT CAST(\"__tyrus_root\".\"createdAt\" AS CHAR) AS \"__tyrus_instant__createdAt\" FROM \"users\""
        );
    }
}
