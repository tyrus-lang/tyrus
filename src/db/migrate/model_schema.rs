use super::*;

pub(in crate::db) fn generate_model_schema(
    items: &[tyrus_core::ast::Item],
) -> Result<String, String> {
    use tyrus_core::ast::{ExprKind, Item, Type};
    let mut models = Vec::new();
    for item in items {
        let Item::InterfaceDecl {
            name,
            fields,
            db_model: Some(model),
            ..
        } = item
        else {
            continue;
        };
        let has_declared_fields = model.fields.iter().any(|metadata| {
            !model.timestamps || !matches!(metadata.field.as_str(), "createdAt" | "updatedAt")
        });
        if !has_declared_fields {
            return Err(format!("model `{name}` has no fields to generate"));
        }
        validate_model_column_names(name, model)?;
        let mut columns = Vec::new();
        let mut primary_keys = Vec::new();
        let mut foreign_keys = Vec::new();
        for metadata in &model.fields {
            let field = fields
                .iter()
                .find(|field| field.name == metadata.field)
                .ok_or_else(|| format!("model `{name}` field metadata is inconsistent"))?;
            let nullable = matches!(field.ty, Type::Optional(_));
            let base_type = match &field.ty {
                Type::Optional(inner) => inner.as_ref(),
                ty => ty,
            };
            let sql_type = sqlite_type(base_type).ok_or_else(|| {
                format!(
                    "model `{name}` field `{}` uses a type that cannot be represented in SQLite",
                    field.name
                )
            })?;
            let mut column = format!("{} {sql_type}", quote_identifier(&metadata.column));
            if metadata.id {
                primary_keys.push(quote_identifier(&metadata.column));
            }
            if metadata.unique {
                column.push_str(" UNIQUE");
            }
            if let Some(default) = &metadata.default_value {
                let (ExprKind::StrLit(_)
                | ExprKind::IntLit(_)
                | ExprKind::UIntLit(_)
                | ExprKind::FloatLit(_)
                | ExprKind::DecimalLit(_)
                | ExprKind::BoolLit(_)) = default.kind()
                else {
                    return Err(format!(
                        "model `{name}` field `{}` has an unsupported default",
                        field.name
                    ));
                };
                column.push_str(" DEFAULT ");
                column.push_str(&sqlite_default(default.kind()));
            } else if model.timestamps
                && matches!(metadata.field.as_str(), "createdAt" | "updatedAt")
            {
                column.push_str(" DEFAULT CURRENT_TIMESTAMP");
            }
            if !nullable {
                column.push_str(" NOT NULL");
            }
            if let Some((table, target)) = &metadata.foreign_key {
                foreign_keys.push(format!(
                    "FOREIGN KEY ({}) REFERENCES {} ({})",
                    quote_identifier(&metadata.column),
                    quote_identifier(table),
                    quote_identifier(target)
                ));
            }
            columns.push(column);
        }
        let mut definitions = columns;
        if !primary_keys.is_empty() {
            definitions.push(format!("PRIMARY KEY ({})", primary_keys.join(", ")));
        }
        definitions.extend(foreign_keys);
        let mut model_sql = format!(
            "CREATE TABLE {} (\n    {}\n);",
            quote_identifier(&model.table),
            definitions.join(",\n    ")
        );
        if model.timestamps {
            model_sql.push_str("\n\n");
            model_sql.push_str(&sqlite_timestamps_trigger_sql(model)?);
        }
        models.push((model.table.clone(), model_sql));
    }
    models.sort_by(|left, right| left.0.cmp(&right.0));
    let mut seen = Vec::new();
    for (table, _) in &models {
        if seen
            .iter()
            .any(|existing: &&String| sqlite_identifier_eq(existing, table))
        {
            return Err(format!("multiple models map to table `{table}`"));
        }
        seen.push(table);
    }
    Ok(models
        .into_iter()
        .map(|(_, sql)| sql)
        .collect::<Vec<_>>()
        .join("\n\n")
        + "\n")
}

pub(in crate::db) fn generate_server_model_schema(
    items: &[tyrus_core::ast::Item],
    backend: &tyrus_core::db::DatabaseBackend,
) -> Result<String, String> {
    use tyrus_core::ast::{ExprKind, Item, Type};
    use tyrus_core::db::DatabaseBackend;

    let mut models = Vec::new();
    for item in items {
        let Item::InterfaceDecl {
            name,
            fields,
            db_model: Some(model),
            ..
        } = item
        else {
            continue;
        };
        if model.fields.is_empty() {
            return Err(format!("model `{name}` has no fields to generate"));
        }
        validate_model_column_names(name, model)?;

        let quote = |identifier: &str| quote_identifier_for_backend(identifier, backend);
        let mut columns = Vec::new();
        let mut primary_keys = Vec::new();
        let mut foreign_keys = Vec::new();
        for metadata in &model.fields {
            let field = fields
                .iter()
                .find(|field| field.name == metadata.field)
                .ok_or_else(|| format!("model `{name}` field metadata is inconsistent"))?;
            let nullable = matches!(field.ty, Type::Optional(_));
            let base_type = match &field.ty {
                Type::Optional(inner) => inner.as_ref(),
                ty => ty,
            };
            let sql_type = server_model_field_sql_type(metadata, base_type, backend).ok_or_else(|| {
                format!(
                    "model `{name}` field `{}` uses a type without a lossless mapping for this server backend",
                    field.name
                )
            })?;
            if *backend == DatabaseBackend::Mysql
                && (metadata.id || metadata.unique)
                && matches!(base_type, Type::Str | Type::Char | Type::Array(_))
            {
                return Err(format!(
                    "model `{name}` field `{}` cannot use a MySQL text or binary column as a key without a declared bounded type",
                    field.name
                ));
            }
            if *backend == DatabaseBackend::Mysql
                && metadata.default_value.is_some()
                && matches!(base_type, Type::Str | Type::Char | Type::Array(_))
            {
                return Err(format!(
                    "model `{name}` field `{}` uses a text or binary default that cannot be generated portably for MySQL",
                    field.name
                ));
            }
            if metadata.id && nullable {
                return Err(format!(
                    "model `{name}` primary-key field `{}` cannot be optional",
                    field.name
                ));
            }
            let strategy = if metadata.id {
                Some(resolve_server_id_strategy(metadata, base_type, backend)?)
            } else {
                None
            };
            let mut column = format!("{} {sql_type}", quote(&metadata.column));
            match strategy.as_deref() {
                Some("identity") => column.push_str(" GENERATED BY DEFAULT AS IDENTITY"),
                Some("auto_increment") => column.push_str(" AUTO_INCREMENT"),
                Some("uuid") if *backend == DatabaseBackend::Postgresql => {
                    column.push_str(" DEFAULT gen_random_uuid()")
                }
                Some("uuid") => column.push_str(" DEFAULT (UUID())"),
                Some("uuid7") => column.push_str(" DEFAULT uuidv7()"),
                Some("uuid_short") => column.push_str(" DEFAULT (UUID_SHORT())"),
                Some(_) => unreachable!("ID strategies are validated before SQL generation"),
                None => {}
            }
            if metadata.id {
                primary_keys.push(quote(&metadata.column));
            }
            if metadata.unique {
                column.push_str(" UNIQUE");
            }
            if let Some(default) = &metadata.default_value {
                let (ExprKind::StrLit(_)
                | ExprKind::IntLit(_)
                | ExprKind::UIntLit(_)
                | ExprKind::FloatLit(_)
                | ExprKind::DecimalLit(_)
                | ExprKind::BoolLit(_)) = default.kind()
                else {
                    return Err(format!(
                        "model `{name}` field `{}` has an unsupported default",
                        field.name
                    ));
                };
                column.push_str(" DEFAULT ");
                column.push_str(&server_default(default.kind(), backend));
            } else if model.timestamps
                && matches!(metadata.field.as_str(), "createdAt" | "updatedAt")
            {
                column.push_str(if *backend == DatabaseBackend::Mysql {
                    " DEFAULT CURRENT_TIMESTAMP(3)"
                } else {
                    " DEFAULT CURRENT_TIMESTAMP"
                });
            }
            if !nullable {
                column.push_str(" NOT NULL");
            }
            if let Some((table, target)) = &metadata.foreign_key {
                foreign_keys.push(format!(
                    "FOREIGN KEY ({}) REFERENCES {} ({})",
                    quote(&metadata.column),
                    quote(table),
                    quote(target)
                ));
            }
            columns.push(column);
        }
        if primary_keys.is_empty() {
            return Err(format!(
                "model `{name}` must declare at least one `@id` field"
            ));
        }
        let mut definitions = columns;
        definitions.push(format!("PRIMARY KEY ({})", primary_keys.join(", ")));
        definitions.extend(foreign_keys);
        models.push((
            model.table.clone(),
            format!(
                "CREATE TABLE {} (\n    {}\n){};",
                quote(&model.table),
                definitions.join(",\n    "),
                if *backend == DatabaseBackend::Mysql {
                    " ENGINE=InnoDB"
                } else {
                    ""
                }
            ),
        ));
    }
    models.sort_by(|left, right| left.0.cmp(&right.0));
    for index in 0..models.len() {
        if models[..index]
            .iter()
            .any(|(existing, _)| identifier_matches(existing, &models[index].0, true))
        {
            return Err(format!(
                "multiple models map to table `{}`",
                models[index].0
            ));
        }
    }
    if models.is_empty() {
        return Ok(String::new());
    }
    Ok(models
        .into_iter()
        .map(|(_, sql)| sql)
        .collect::<Vec<_>>()
        .join("\n\n")
        + "\n")
}

pub(in crate::db) fn server_sql_type(
    ty: &tyrus_core::ast::Type,
    backend: &tyrus_core::db::DatabaseBackend,
) -> Option<&'static str> {
    use tyrus_core::ast::Type;
    use tyrus_core::db::DatabaseBackend;
    Some(match ty {
        Type::I8 | Type::I16 => "SMALLINT",
        Type::I32 => "INTEGER",
        Type::I64 => "BIGINT",
        Type::U64 if *backend == DatabaseBackend::Mysql => "BIGINT UNSIGNED",
        Type::Named(name) if name == "Uuid" && *backend == DatabaseBackend::Mysql => "CHAR(36)",
        Type::Named(name) if name == "Uuid" || name == "Uuid7" => match backend {
            DatabaseBackend::Postgresql => "UUID",
            _ => return None,
        },
        Type::Named(name) if name == "Instant" => match backend {
            DatabaseBackend::Postgresql => "TIMESTAMPTZ",
            DatabaseBackend::Mysql => "DATETIME(3)",
            _ => return None,
        },
        Type::Bool if *backend == DatabaseBackend::Postgresql => "BOOLEAN",
        Type::F32 => "REAL",
        Type::F64 => match backend {
            DatabaseBackend::Postgresql => "DOUBLE PRECISION",
            DatabaseBackend::Mysql => "DOUBLE",
            _ => return None,
        },
        Type::Str | Type::Char => match backend {
            DatabaseBackend::Postgresql => "TEXT",
            DatabaseBackend::Mysql => "LONGTEXT",
            _ => return None,
        },
        Type::Array(element) if **element == Type::U8 => match backend {
            DatabaseBackend::Postgresql => "BYTEA",
            DatabaseBackend::Mysql => "BLOB",
            _ => return None,
        },
        _ => return None,
    })
}

fn server_model_field_sql_type(
    metadata: &tyrus_core::ast::DbModelFieldMetadata,
    ty: &tyrus_core::ast::Type,
    backend: &tyrus_core::db::DatabaseBackend,
) -> Option<&'static str> {
    use tyrus_core::db::DatabaseBackend;
    match metadata.id_strategy.as_deref() {
        Some("uuid") if *backend == DatabaseBackend::Mysql => Some("CHAR(36)"),
        Some("uuid" | "uuid7") if *backend == DatabaseBackend::Postgresql => Some("UUID"),
        Some("uuid_short") if *backend == DatabaseBackend::Mysql => Some("BIGINT UNSIGNED"),
        _ => server_sql_type(ty, backend),
    }
}

fn resolve_server_id_strategy(
    metadata: &tyrus_core::ast::DbModelFieldMetadata,
    ty: &tyrus_core::ast::Type,
    backend: &tyrus_core::db::DatabaseBackend,
) -> Result<String, String> {
    use tyrus_core::ast::Type;
    use tyrus_core::db::DatabaseBackend;

    let strategy = metadata.id_strategy.as_deref().unwrap_or(match backend {
        DatabaseBackend::Postgresql => "identity",
        DatabaseBackend::Mysql => "auto_increment",
        _ => return Ok(String::new()),
    });
    let valid = match (backend, strategy) {
        (DatabaseBackend::Mysql, "auto_increment") => *ty == Type::I64,
        (DatabaseBackend::Mysql, "uuid") => matches!(ty, Type::Named(name) if name == "Uuid"),
        (DatabaseBackend::Mysql, "uuid_short") => *ty == Type::U64,
        (DatabaseBackend::Postgresql, "identity") => *ty == Type::I64,
        (DatabaseBackend::Postgresql, "uuid") => matches!(ty, Type::Named(name) if name == "Uuid"),
        (DatabaseBackend::Postgresql, "uuid7") => {
            matches!(ty, Type::Named(name) if name == "Uuid7")
        }
        _ => false,
    };
    if !valid {
        let choices = match backend {
            DatabaseBackend::Mysql => {
                "`auto_increment` with a signed integer, `uuid` with `Uuid`, or `uuid_short` with `u64`"
            }
            DatabaseBackend::Postgresql => {
                "`identity` with a signed integer, `uuid` with `Uuid`, or `uuid7` with `Uuid7`"
            }
            _ => "a backend-supported strategy",
        };
        return Err(format!(
            "model ID field `{}` has an unsupported strategy `{strategy}` or incompatible type `{}`; use {choices}",
            metadata.field, metadata.ty
        ));
    }
    if metadata.default_value.is_some() {
        return Err(format!(
            "model ID field `{}` cannot combine an ID generation strategy with `@default`",
            metadata.field
        ));
    }
    Ok(strategy.to_string())
}

pub(in crate::db) fn server_default(
    kind: &tyrus_core::ast::ExprKind,
    backend: &tyrus_core::db::DatabaseBackend,
) -> String {
    use tyrus_core::ast::ExprKind;
    match kind {
        ExprKind::StrLit(value) => format!("'{}'", value.replace('\'', "''")),
        ExprKind::IntLit(value) => value.to_string(),
        ExprKind::UIntLit(value) => value.to_string(),
        ExprKind::FloatLit(value) => value.text().to_string(),
        ExprKind::DecimalLit(value) => value.text().to_string(),
        ExprKind::BoolLit(value) => match backend {
            tyrus_core::db::DatabaseBackend::Postgresql => value.to_string().to_uppercase(),
            tyrus_core::db::DatabaseBackend::Mysql => if *value { "1" } else { "0" }.to_string(),
            _ => unreachable!("server model generation only accepts server backends"),
        },
        _ => unreachable!("model defaults are validated as literals"),
    }
}

pub(in crate::db) fn quote_identifier_for_backend(
    identifier: &str,
    backend: &tyrus_core::db::DatabaseBackend,
) -> String {
    match backend {
        tyrus_core::db::DatabaseBackend::Mysql => {
            format!("`{}`", identifier.replace('`', "``"))
        }
        _ => quote_identifier(identifier),
    }
}

fn identifier_matches(left: &str, right: &str, case_insensitive: bool) -> bool {
    if case_insensitive {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

pub(in crate::db) fn validate_initial_model_foreign_keys(
    items: &[tyrus_core::ast::Item],
    case_insensitive_identifiers: bool,
) -> Result<(), String> {
    use tyrus_core::ast::{Field, Item, Type};

    type Model<'a> = (&'a str, &'a [Field], &'a tyrus_core::ast::DbModelMetadata);
    let mut models = Vec::<Model<'_>>::new();
    for item in items {
        if let Item::InterfaceDecl {
            name,
            fields,
            db_model: Some(model),
            ..
        } = item
        {
            if models.iter().any(|(_, _, existing)| {
                identifier_matches(&existing.table, &model.table, case_insensitive_identifiers)
            }) {
                return Err(format!("multiple models map to table `{}`", model.table));
            }
            models.push((name.as_str(), fields.as_slice(), model));
        }
    }

    for (_, source_fields, source_model) in &models {
        for source_metadata in &source_model.fields {
            let Some((target_table, target_column)) = &source_metadata.foreign_key else {
                continue;
            };
            let source_field = source_fields
                .iter()
                .find(|field| field.name == source_metadata.field)
                .ok_or_else(|| {
                    format!(
                        "model field `{}.{}` has inconsistent foreign-key metadata",
                        source_model.table, source_metadata.field
                    )
                })?;
            let (target_name, target_fields, target_model) = models
                .iter()
                .find(|(_, _, model)| {
                    identifier_matches(
                        &model.table,
                        target_table,
                        case_insensitive_identifiers,
                    )
                })
                .copied()
                .ok_or_else(|| {
                    format!(
                        "model field `{}.{}` references table `{target_table}`, but that table has no project Model for the initial migration",
                        source_model.table, source_metadata.field
                    )
                })?;
            let target_metadata = target_model
                .fields
                .iter()
                .find(|field| {
                    identifier_matches(
                        &field.column,
                        target_column,
                        case_insensitive_identifiers,
                    )
                })
                .ok_or_else(|| {
                    format!(
                        "model field `{}.{}` references missing Model column `{target_table}.{target_column}`",
                        source_model.table, source_metadata.field
                    )
                })?;
            let target_field = target_fields
                .iter()
                .find(|field| field.name == target_metadata.field)
                .ok_or_else(|| {
                    format!(
                        "model `{target_name}` has inconsistent metadata for column `{target_column}`"
                    )
                })?;
            let target_id_count = target_model.fields.iter().filter(|field| field.id).count();
            if !target_metadata.unique && !(target_metadata.id && target_id_count == 1) {
                return Err(format!(
                    "model field `{}.{}` references `{target_table}.{target_column}`, which must be a single-column primary key or unique column",
                    source_model.table, source_metadata.field
                ));
            }
            let source_type = match &source_field.ty {
                Type::Optional(inner) => inner.as_ref(),
                ty => ty,
            };
            let target_type = match &target_field.ty {
                Type::Optional(inner) => inner.as_ref(),
                ty => ty,
            };
            if source_type != target_type {
                return Err(format!(
                    "model field `{}.{}` has type `{source_type}` but referenced Model column `{target_table}.{target_column}` has type `{target_type}`",
                    source_model.table, source_metadata.field
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn generate_incremental_schema(
    items: &[tyrus_core::ast::Item],
    existing_tables: &[tyrus_core::db::DbTableSchema],
    view_sql: &[(String, String)],
    index_sql_by_table: &BTreeMap<String, Vec<String>>,
    trigger_sql_by_table: &BTreeMap<String, Vec<String>>,
) -> Result<String, String> {
    use tyrus_core::ast::{ExprKind, Item, Type};
    let tables = existing_tables.iter().collect::<Vec<_>>();
    let mut changes = Vec::new();
    let mut views_dropped = false;
    let mut seen = Vec::new();
    for item in items {
        let Item::InterfaceDecl {
            name,
            fields,
            db_model: Some(model),
            ..
        } = item
        else {
            continue;
        };
        validate_model_column_names(name, model)?;
        if seen
            .iter()
            .any(|table: &&str| sqlite_identifier_eq(table, &model.table))
        {
            return Err(format!("multiple models map to table `{}`", model.table));
        }
        seen.push(model.table.as_str());
        let Some(existing) = tables
            .iter()
            .find(|table| sqlite_identifier_eq(&table.name, &model.table))
            .copied()
        else {
            changes.push(generate_model_schema(std::slice::from_ref(item))?);
            continue;
        };
        let timestamp_trigger = sqlite_timestamps_trigger_name(&model.table);
        let existing_triggers = lookup_sqlite_identifier(trigger_sql_by_table, &model.table)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let has_timestamp_trigger = existing_triggers.iter().any(|sql| {
            sql.to_ascii_lowercase()
                .contains(&timestamp_trigger.to_ascii_lowercase())
        });
        if !model.timestamps && has_timestamp_trigger {
            changes.push(format!(
                "DROP TRIGGER {};",
                quote_identifier(&timestamp_trigger)
            ));
        }
        let expected_primary = model
            .fields
            .iter()
            .filter(|field| field.id)
            .map(|field| field.column.clone())
            .collect::<Vec<_>>();
        let actual_primary = existing.primary_key.clone();
        let primary_changed = !sqlite_identifier_list_eq(&expected_primary, &actual_primary);
        let expected_unique = model
            .fields
            .iter()
            .filter(|field| field.unique)
            .map(|field| vec![field.column.clone()])
            .collect::<Vec<_>>();
        let actual_unique = existing.unique.clone();
        let unique_changed = !sqlite_unique_keys_eq(&expected_unique, &actual_unique);
        let expected_foreign = model
            .fields
            .iter()
            .filter_map(|field| {
                field
                    .foreign_key
                    .as_ref()
                    .map(|(table, column)| (field.column.clone(), table.clone(), column.clone()))
            })
            .collect::<BTreeSet<_>>();
        let actual_foreign = existing
            .foreign_keys
            .iter()
            .flat_map(|foreign_key| {
                foreign_key
                    .columns
                    .iter()
                    .zip(&foreign_key.referenced_columns)
                    .map(move |(column, target)| {
                        (
                            column.clone(),
                            foreign_key.referenced_table.clone(),
                            target.clone(),
                        )
                    })
            })
            .collect::<BTreeSet<_>>();
        let foreign_changed = expected_foreign.len() != actual_foreign.len()
            || expected_foreign.iter().any(|(column, table, target)| {
                !actual_foreign
                    .iter()
                    .any(|(actual_column, actual_table, actual_target)| {
                        sqlite_identifier_eq(column, actual_column)
                            && sqlite_identifier_eq(table, actual_table)
                            && sqlite_identifier_eq(target, actual_target)
                    })
            });
        let existing_columns = existing
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column))
            .collect::<Vec<_>>();
        let model_columns = model
            .fields
            .iter()
            .map(|field| field.column.as_str())
            .collect::<Vec<_>>();
        let removed_columns = existing
            .columns
            .iter()
            .filter(|column| {
                !model_columns
                    .iter()
                    .any(|name| sqlite_identifier_eq(name, &column.name))
            })
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>();
        let missing_timestamp_columns = model.timestamps
            && model.fields.iter().any(|field| {
                matches!(field.field.as_str(), "createdAt" | "updatedAt")
                    && !existing_columns
                        .iter()
                        .any(|(name, _)| sqlite_identifier_eq(name, &field.column))
            });
        if !removed_columns.is_empty()
            || primary_changed
            || unique_changed
            || foreign_changed
            || missing_timestamp_columns
        {
            if model.fields.iter().any(|field| {
                !existing_columns
                    .iter()
                    .any(|(name, _)| sqlite_identifier_eq(name, &field.column))
                    && !(model.timestamps
                        && matches!(field.field.as_str(), "createdAt" | "updatedAt"))
            }) {
                return Err(format!(
                    "model `{name}` combines a column change with a primary-key or unique-constraint change in table `{}`; split this into separate reviewed migrations",
                    model.table
                ));
            }
            for metadata in &model.fields {
                let field = fields
                    .iter()
                    .find(|field| field.name == metadata.field)
                    .ok_or_else(|| format!("model `{name}` field metadata is inconsistent"))?;
                let nullable = matches!(field.ty, Type::Optional(_));
                let base_type = match &field.ty {
                    Type::Optional(inner) => inner.as_ref(),
                    ty => ty,
                };
                let sql_type = sqlite_type(base_type).ok_or_else(|| {
                    format!(
                        "model `{name}` field `{}` uses a type that cannot be represented in SQLite",
                        field.name
                    )
                })?;
                let expected_type = match sql_type {
                    "INTEGER" => "i64",
                    "REAL" => "f64",
                    "BLOB" => "bytes",
                    "NUMERIC" | "TEXT" => "string",
                    _ => unreachable!("SQLite type has a logical mapping"),
                };
                let expected_default = metadata
                    .default_value
                    .as_ref()
                    .map(|value| sqlite_default(value.kind()));
                let column = existing_columns
                    .iter()
                    .find(|(name, _)| sqlite_identifier_eq(name, &metadata.column))
                    .map(|(_, column)| *column)
                    .ok_or_else(|| {
                        format!(
                            "could not inspect existing column `{}.{}`",
                            model.table, metadata.column
                        )
                    })?;
                if column.logical_type != expected_type
                    || column.nullable != nullable
                    || column.default != expected_default
                {
                    return Err(format!(
                        "model `{name}` also changes existing column `{}.{}`; split destructive changes into separate reviewed migrations",
                        model.table, metadata.column
                    ));
                }
            }
            if lookup_sqlite_identifier(trigger_sql_by_table, &model.table)
                .into_iter()
                .flatten()
                .any(|sql| trigger_references_removed_column(sql, &removed_columns))
            {
                return Err(format!(
                    "model `{name}` removes a column referenced by a trigger on table `{}`; revise the trigger or create a manual migration",
                    model.table
                ));
            }
            if !view_sql.is_empty() && !views_dropped {
                changes.extend(
                    view_sql
                        .iter()
                        .map(|(name, _)| format!("DROP VIEW {};", quote_identifier(name))),
                );
                views_dropped = true;
            }
            if existing_tables.iter().any(|table| {
                !sqlite_identifier_eq(&table.name, &model.table)
                    && table.foreign_keys.iter().any(|foreign_key| {
                        sqlite_identifier_eq(&foreign_key.referenced_table, &model.table)
                    })
            }) {
                return Err(format!(
                    "model `{name}` changes table `{}` while another table references it; create a manual migration",
                    model.table
                ));
            }
            let custom_indexes = existing
                .indexes
                .iter()
                .filter(|index| {
                    !(index.name.starts_with("sqlite_autoindex_")
                        && index.unique
                        && !index.partial
                        && !index.expression)
                })
                .collect::<Vec<_>>();
            if custom_indexes.iter().any(|index| {
                !removed_columns.is_empty()
                    && (index.partial
                        || index.expression
                        || index.columns.iter().any(|column| {
                            removed_columns
                                .iter()
                                .any(|removed| sqlite_identifier_eq(column, removed))
                        }))
            }) {
                return Err(format!(
                    "model `{name}` removes columns referenced by custom indexes on table `{}`; create a manual migration",
                    model.table
                ));
            }
            let index_sql = lookup_sqlite_identifier(index_sql_by_table, &model.table)
                .map(Vec::as_slice)
                .unwrap_or_default();
            if custom_indexes.len() != index_sql.len() {
                return Err(format!(
                    "model `{name}` cannot safely recover all custom index definitions for table `{}`; create a manual migration",
                    model.table
                ));
            }
            let retained = model_columns
                .iter()
                .filter(|column| {
                    existing_columns
                        .iter()
                        .any(|(existing, _)| sqlite_identifier_eq(existing, column))
                })
                .copied()
                .collect::<Vec<_>>();
            if retained.is_empty() {
                return Err(format!(
                    "model `{name}` removes every existing column from table `{}`; row preservation cannot be guaranteed",
                    model.table
                ));
            }
            let generated = generate_model_schema(std::slice::from_ref(item))?;
            let generated_table = if model.timestamps {
                generated
                    .split_once("\n\nCREATE TRIGGER ")
                    .map_or(generated.as_str(), |(table, _)| table)
            } else {
                generated.as_str()
            };
            let old_table = format!("CREATE TABLE {}", quote_identifier(&model.table));
            let temporary_name = format!("__tyrus_migrate_{}", model.table);
            let new_table = format!("CREATE TABLE {}", quote_identifier(&temporary_name));
            let create_table = generated_table
                .strip_prefix(&old_table)
                .ok_or_else(|| format!("could not generate rebuilt table for model `{name}`"))?;
            changes.push(format!("{new_table}{create_table}"));
            let retained_columns = retained
                .iter()
                .map(|column| quote_identifier(column))
                .collect::<Vec<_>>()
                .join(", ");
            changes.push(format!(
                "INSERT INTO {} ({retained_columns}) SELECT {retained_columns} FROM {};",
                quote_identifier(&temporary_name),
                quote_identifier(&model.table)
            ));
            changes.push(format!("DROP TABLE {};", quote_identifier(&model.table)));
            changes.push(format!(
                "ALTER TABLE {} RENAME TO {};",
                quote_identifier(&temporary_name),
                quote_identifier(&model.table)
            ));
            changes.extend(index_sql.iter().map(|sql| {
                let sql = sql.trim_end().trim_end_matches(';').trim_end();
                format!("{sql};")
            }));
            changes.extend(
                lookup_sqlite_identifier(trigger_sql_by_table, &model.table)
                    .into_iter()
                    .flatten()
                    .filter(|sql| {
                        let is_timestamp_trigger = sql
                            .to_ascii_lowercase()
                            .contains(&timestamp_trigger.to_ascii_lowercase());
                        !(is_timestamp_trigger && (!model.timestamps || missing_timestamp_columns))
                    })
                    .map(|sql| {
                        let sql = sql.trim_end().trim_end_matches(';').trim_end();
                        format!("{sql};")
                    }),
            );
            if model.timestamps && (!has_timestamp_trigger || missing_timestamp_columns) {
                changes.push(sqlite_timestamps_trigger_sql(model)?);
            }
            continue;
        }
        for metadata in &model.fields {
            let field = fields
                .iter()
                .find(|field| field.name == metadata.field)
                .ok_or_else(|| format!("model `{name}` field metadata is inconsistent"))?;
            let nullable = matches!(field.ty, Type::Optional(_));
            let base_type = match &field.ty {
                Type::Optional(inner) => inner.as_ref(),
                ty => ty,
            };
            let sql_type = sqlite_type(base_type).ok_or_else(|| {
                format!(
                    "model `{name}` field `{}` uses a type that cannot be represented in SQLite",
                    field.name
                )
            })?;
            if let Some(column) = existing_columns
                .iter()
                .find(|(existing, _)| sqlite_identifier_eq(existing, &metadata.column))
                .map(|(_, column)| *column)
            {
                let expected_type = match sql_type {
                    "INTEGER" => "i64",
                    "REAL" => "f64",
                    "BLOB" => "bytes",
                    "NUMERIC" | "TEXT" => "string",
                    _ => unreachable!("SQLite type has a logical mapping"),
                };
                let expected_default = metadata
                    .default_value
                    .as_ref()
                    .map(|value| {
                        let (ExprKind::StrLit(_)
                        | ExprKind::IntLit(_)
                        | ExprKind::UIntLit(_)
                        | ExprKind::FloatLit(_)
                        | ExprKind::DecimalLit(_)
                        | ExprKind::BoolLit(_)) = value.kind()
                        else {
                            unreachable!("model defaults are validated as literals")
                        };
                        sqlite_default(value.kind())
                    })
                    .or_else(|| {
                        (model.timestamps
                            && matches!(metadata.field.as_str(), "createdAt" | "updatedAt"))
                        .then(|| "CURRENT_TIMESTAMP".to_string())
                    });
                if column.logical_type != expected_type
                    || column.nullable != nullable
                    || column.default.as_deref() != expected_default.as_deref()
                {
                    return Err(format!(
                        "model `{name}` changes existing column `{}.{}`; create a manual migration",
                        model.table, metadata.column
                    ));
                }
                continue;
            }
            if !nullable
                && metadata.default_value.is_none()
                && !(model.timestamps
                    && matches!(metadata.field.as_str(), "createdAt" | "updatedAt"))
            {
                return Err(format!(
                    "model `{name}` adds required column `{}.{}` without a default; existing rows would be invalid",
                    model.table, metadata.column
                ));
            }
            if metadata.id || metadata.unique || metadata.foreign_key.is_some() {
                return Err(format!(
                    "model `{name}` adds constrained column `{}.{}`; create a manual migration",
                    model.table, metadata.column
                ));
            }
            let mut definition = format!("{} {sql_type}", quote_identifier(&metadata.column));
            if let Some(value) = &metadata.default_value {
                definition.push_str(" DEFAULT ");
                definition.push_str(&sqlite_default(value.kind()));
            } else if model.timestamps
                && matches!(metadata.field.as_str(), "createdAt" | "updatedAt")
            {
                definition.push_str(" DEFAULT CURRENT_TIMESTAMP");
            }
            if !nullable {
                definition.push_str(" NOT NULL");
            }
            changes.push(format!(
                "ALTER TABLE {} ADD COLUMN {definition};",
                quote_identifier(&model.table)
            ));
        }
        if model.timestamps && !has_timestamp_trigger {
            changes.push(sqlite_timestamps_trigger_sql(model)?);
        }
    }
    if changes.is_empty() {
        return Err("models match the database schema; no migration is needed".to_string());
    }
    if views_dropped {
        changes.extend(view_sql.iter().map(|(_, sql)| {
            let sql = sql.trim_end().trim_end_matches(';').trim_end();
            format!("{sql};")
        }));
    }
    Ok(changes.join("\n\n") + "\n")
}

pub(super) struct SqliteRebuildMetadata {
    pub(super) schema_sql: Vec<String>,
    pub(super) views: Vec<(String, String)>,
    pub(super) indexes_by_table: BTreeMap<String, Vec<String>>,
    pub(super) triggers_by_table: BTreeMap<String, Vec<String>>,
}

pub(super) fn inspect_sqlite_rebuild_metadata(
    database_path: &Path,
) -> Result<SqliteRebuildMetadata, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not inspect SQLite dependencies".to_string())?;
    runtime.block_on(async {
        let options = SqliteConnectOptions::new()
            .filename(database_path)
            .read_only(true);
        let mut connection = SqliteConnection::connect_with(&options)
            .await
            .map_err(|_| "could not inspect SQLite dependencies".to_string())?;
        let schema_rows = sqlx::query(
            "SELECT sql FROM sqlite_schema WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY CASE type WHEN 'table' THEN 0 WHEN 'index' THEN 1 WHEN 'view' THEN 2 WHEN 'trigger' THEN 3 ELSE 4 END, name",
        )
        .fetch_all(&mut connection)
        .await
        .map_err(|_| "could not inspect SQLite schema definitions".to_string())?;
        let mut schema_sql = Vec::with_capacity(schema_rows.len());
        for row in schema_rows {
            schema_sql.push(
                row.try_get("sql")
                    .map_err(|_| "could not read SQLite schema definition".to_string())?,
            );
        }
        let views = sqlx::query(
            "SELECT name, sql FROM sqlite_schema WHERE type = 'view' AND sql IS NOT NULL ORDER BY name",
        )
        .fetch_all(&mut connection)
        .await
        .map_err(|_| "could not inspect SQLite view definitions".to_string())?;
        let mut view_sql = Vec::with_capacity(views.len());
        for view in views {
            let name: String = view
                .try_get("name")
                .map_err(|_| "could not read SQLite view name".to_string())?;
            let sql: String = view
                .try_get("sql")
                .map_err(|_| "could not read SQLite view definition".to_string())?;
            view_sql.push((name, sql));
        }
        let mut index_sql_by_table = BTreeMap::<String, Vec<String>>::new();
        let indexes = sqlx::query(
            "SELECT tbl_name, sql FROM sqlite_schema WHERE type = 'index' AND sql IS NOT NULL ORDER BY tbl_name, name",
        )
        .fetch_all(&mut connection)
        .await
        .map_err(|_| "could not inspect SQLite index definitions".to_string())?;
        for index in indexes {
            let table: String = index
                .try_get("tbl_name")
                .map_err(|_| "could not read SQLite index table".to_string())?;
            let sql: String = index
                .try_get("sql")
                .map_err(|_| "could not read SQLite index definition".to_string())?;
            index_sql_by_table.entry(table).or_default().push(sql);
        }
        let mut trigger_sql_by_table = BTreeMap::<String, Vec<String>>::new();
        let triggers = sqlx::query(
            "SELECT tbl_name, sql FROM sqlite_schema WHERE type = 'trigger' AND sql IS NOT NULL ORDER BY tbl_name, name",
        )
        .fetch_all(&mut connection)
        .await
        .map_err(|_| "could not inspect SQLite trigger definitions".to_string())?;
        for trigger in triggers {
            let table: String = trigger
                .try_get("tbl_name")
                .map_err(|_| "could not read SQLite trigger table".to_string())?;
            let sql: String = trigger
                .try_get("sql")
                .map_err(|_| "could not read SQLite trigger definition".to_string())?;
            trigger_sql_by_table.entry(table).or_default().push(sql);
        }
        connection
            .close()
            .await
            .map_err(|_| "could not close SQLite dependency inspection".to_string())?;
        Ok(SqliteRebuildMetadata {
            schema_sql,
            views: view_sql,
            indexes_by_table: index_sql_by_table,
            triggers_by_table: trigger_sql_by_table,
        })
    })
}

pub(super) fn validate_generated_schema_migration(
    schema_sql: &[String],
    migration_sql: &str,
) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not validate SQLite migration dependencies".to_string())?;
    runtime.block_on(async {
        let options = SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
        let mut connection = SqliteConnection::connect_with(&options)
            .await
            .map_err(|_| "could not initialize SQLite migration validation".to_string())?;
        for statement in schema_sql {
            sqlx::raw_sql(sqlx::AssertSqlSafe(statement.clone()))
                .execute(&mut connection)
                .await
                .map_err(|_| "could not reproduce the current SQLite schema for migration validation".to_string())?;
        }
        sqlx::raw_sql(sqlx::AssertSqlSafe(migration_sql.to_string()))
            .execute(&mut connection)
            .await
            .map_err(|_| "generated migration is incompatible with the existing SQLite schema dependencies; revise affected views or triggers, or create a manual migration".to_string())?;
        validate_sqlite_foreign_keys(&mut connection).await?;

        let views = sqlx::query_scalar::<_, String>(
            "SELECT name FROM sqlite_schema WHERE type = 'view' ORDER BY name",
        )
        .fetch_all(&mut connection)
        .await
        .map_err(|_| "could not inspect validated SQLite views".to_string())?;
        for view in views {
            let sql = format!("EXPLAIN SELECT * FROM {} LIMIT 0", quote_identifier(&view));
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .fetch_all(&mut connection)
                .await
                .map_err(|_| format!("generated migration breaks view `{view}`; revise the view or create a manual migration"))?;
        }

        let trigger_tables = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT tbl_name FROM sqlite_schema WHERE type = 'trigger' ORDER BY tbl_name",
        )
        .fetch_all(&mut connection)
        .await
        .map_err(|_| "could not inspect validated SQLite triggers".to_string())?;
        for table in trigger_tables {
            let columns = sqlx::query(sqlx::AssertSqlSafe(format!(
                "PRAGMA table_info({})",
                quote_identifier(&table)
            )))
                .fetch_all(&mut connection)
                .await
                .map_err(|_| format!("could not inspect trigger target `{table}`"))?;
            let column = columns
                .first()
                .ok_or_else(|| format!("trigger target `{table}` has no columns"))?
                .try_get::<String, _>("name")
                .map_err(|_| format!("could not inspect trigger target `{table}`"))?;
            let target = quote_identifier(&table);
            let column = quote_identifier(&column);
            for event in [
                format!("INSERT INTO {target} DEFAULT VALUES"),
                format!("UPDATE {target} SET {column} = {column} WHERE 0"),
                format!("DELETE FROM {target} WHERE 0"),
            ] {
                sqlx::query(sqlx::AssertSqlSafe(format!("EXPLAIN {event}")))
                    .fetch_all(&mut connection)
                    .await
                    .map_err(|_| format!("generated migration breaks a trigger on `{table}`; revise the trigger or create a manual migration"))?;
            }
        }
        connection
            .close()
            .await
            .map_err(|_| "could not close SQLite migration validation".to_string())
    })
}

async fn validate_sqlite_foreign_keys(connection: &mut SqliteConnection) -> Result<(), String> {
    let tables = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(&mut *connection)
    .await
    .map_err(|_| "could not inspect generated SQLite tables for foreign keys".to_string())?;

    for child_table in &tables {
        let foreign_key_rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "PRAGMA foreign_key_list({})",
            quote_identifier(child_table)
        )))
        .fetch_all(&mut *connection)
        .await
        .map_err(|_| format!("could not inspect foreign keys on table `{child_table}`"))?;
        let mut foreign_keys =
            BTreeMap::<i64, (String, BTreeMap<i64, (String, Option<String>)>)>::new();
        for row in foreign_key_rows {
            let id = row
                .try_get::<i64, _>("id")
                .map_err(|_| format!("could not read foreign keys on table `{child_table}`"))?;
            let sequence = row
                .try_get::<i64, _>("seq")
                .map_err(|_| format!("could not read foreign keys on table `{child_table}`"))?;
            let parent_table = row
                .try_get::<String, _>("table")
                .map_err(|_| format!("could not read foreign keys on table `{child_table}`"))?;
            let source_column = row
                .try_get::<String, _>("from")
                .map_err(|_| format!("could not read foreign keys on table `{child_table}`"))?;
            let target_column = row
                .try_get::<Option<String>, _>("to")
                .map_err(|_| format!("could not read foreign keys on table `{child_table}`"))?;
            let entry = foreign_keys
                .entry(id)
                .or_insert_with(|| (parent_table.clone(), BTreeMap::new()));
            entry.1.insert(sequence, (source_column, target_column));
        }

        for (_, (parent_table, columns)) in foreign_keys {
            let Some(actual_parent) = tables
                .iter()
                .find(|table| table.eq_ignore_ascii_case(&parent_table))
            else {
                return Err(format!(
                    "generated foreign key on `{child_table}` references missing table `{parent_table}`"
                ));
            };
            let parent_columns = sqlx::query(sqlx::AssertSqlSafe(format!(
                "PRAGMA table_info({})",
                quote_identifier(actual_parent)
            )))
            .fetch_all(&mut *connection)
            .await
            .map_err(|_| format!("could not inspect foreign-key target `{parent_table}`"))?;
            let mut primary_key = parent_columns
                .iter()
                .filter_map(|column| {
                    let ordinal = column.try_get::<i64, _>("pk").ok()?;
                    (ordinal > 0).then(|| {
                        column
                            .try_get::<String, _>("name")
                            .ok()
                            .map(|name| (ordinal, name))
                    })?
                })
                .collect::<Vec<_>>();
            primary_key.sort_by_key(|(ordinal, _)| *ordinal);
            let target_columns = if columns.values().all(|(_, target)| target.is_none()) {
                primary_key
                    .iter()
                    .map(|(_, column)| column.clone())
                    .collect::<Vec<_>>()
            } else if columns.values().all(|(_, target)| target.is_some()) {
                columns
                    .values()
                    .map(|(_, target)| target.clone().unwrap())
                    .collect::<Vec<_>>()
            } else {
                return Err(format!(
                    "generated foreign key on `{child_table}` mixes implicit and explicit target columns"
                ));
            };
            if target_columns.len() != columns.len() || target_columns.is_empty() {
                return Err(format!(
                    "generated foreign key on `{child_table}` does not match a key on `{parent_table}`"
                ));
            }
            let child_columns = sqlx::query(sqlx::AssertSqlSafe(format!(
                "PRAGMA table_info({})",
                quote_identifier(child_table)
            )))
            .fetch_all(&mut *connection)
            .await
            .map_err(|_| format!("could not inspect foreign-key source `{child_table}`"))?;
            for ((source_column, _), target_column) in columns.values().zip(&target_columns) {
                let source_declared_type = child_columns
                    .iter()
                    .find(|column| {
                        column
                            .try_get::<String, _>("name")
                            .is_ok_and(|name| name.eq_ignore_ascii_case(source_column))
                    })
                    .ok_or_else(|| {
                        format!(
                            "generated foreign key on `{child_table}` references missing source column `{source_column}`"
                        )
                    })?
                    .try_get::<String, _>("type")
                    .map_err(|_| format!("could not inspect foreign-key source `{child_table}.{source_column}`"))?;
                let target_declared_type = parent_columns
                    .iter()
                    .find(|column| {
                        column
                            .try_get::<String, _>("name")
                            .is_ok_and(|name| name.eq_ignore_ascii_case(target_column))
                    })
                    .ok_or_else(|| {
                        format!(
                            "generated foreign key on `{child_table}` references missing column `{parent_table}.{target_column}`"
                        )
                    })?
                    .try_get::<String, _>("type")
                    .map_err(|_| format!("could not inspect foreign-key target `{parent_table}.{target_column}`"))?;
                if sqlite_affinity(&source_declared_type) != sqlite_affinity(&target_declared_type)
                {
                    return Err(format!(
                        "generated foreign key on `{child_table}.{source_column}` has a different SQLite type affinity than `{parent_table}.{target_column}`"
                    ));
                }
            }
            let target_is_primary_key = primary_key.len() == target_columns.len()
                && primary_key
                    .iter()
                    .zip(&target_columns)
                    .all(|((_, primary), target)| primary.eq_ignore_ascii_case(target));
            let target_is_unique = if target_is_primary_key {
                true
            } else {
                let indexes = sqlx::query(sqlx::AssertSqlSafe(format!(
                    "PRAGMA index_list({})",
                    quote_identifier(actual_parent)
                )))
                .fetch_all(&mut *connection)
                .await
                .map_err(|_| format!("could not inspect unique keys on `{parent_table}`"))?;
                let mut found = false;
                for index in indexes {
                    if index.try_get::<i64, _>("unique").unwrap_or_default() == 0
                        || index.try_get::<i64, _>("partial").unwrap_or_default() != 0
                    {
                        continue;
                    }
                    let index_name = index.try_get::<String, _>("name").map_err(|_| {
                        format!("could not inspect unique keys on `{parent_table}`")
                    })?;
                    let index_columns = sqlx::query(sqlx::AssertSqlSafe(format!(
                        "PRAGMA index_info({})",
                        quote_identifier(&index_name)
                    )))
                    .fetch_all(&mut *connection)
                    .await
                    .map_err(|_| format!("could not inspect unique index `{index_name}`"))?
                    .into_iter()
                    .map(|column| column.try_get::<Option<String>, _>("name").unwrap_or(None))
                    .collect::<Option<Vec<_>>>();
                    if index_columns.is_some_and(|index_columns| {
                        index_columns.len() == target_columns.len()
                            && index_columns.iter().zip(&target_columns).all(
                                |(index_column, target_column)| {
                                    index_column.eq_ignore_ascii_case(target_column)
                                },
                            )
                    }) {
                        found = true;
                        break;
                    }
                }
                found
            };
            if !target_is_unique {
                return Err(format!(
                    "generated foreign key on `{child_table}` references `{parent_table}({})`, which is not a primary key or unique key",
                    target_columns.join(", ")
                ));
            }
        }
    }

    sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&mut *connection)
        .await
        .map_err(|_| {
            "generated SQLite schema contains an invalid foreign-key definition".to_string()
        })?;
    Ok(())
}

fn sqlite_affinity(declared_type: &str) -> &'static str {
    let declared_type = declared_type.to_ascii_uppercase();
    if declared_type.contains("INT") {
        "INTEGER"
    } else if declared_type.contains("CHAR")
        || declared_type.contains("CLOB")
        || declared_type.contains("TEXT")
    {
        "TEXT"
    } else if declared_type.contains("BLOB") || declared_type.is_empty() {
        "BLOB"
    } else if declared_type.contains("REAL")
        || declared_type.contains("FLOA")
        || declared_type.contains("DOUB")
    {
        "REAL"
    } else {
        "NUMERIC"
    }
}

pub(in crate::db) fn diff_models(
    project_root: &Path,
    snapshot: &tyrus_core::db::DbSchemaSnapshot,
) -> Result<(), String> {
    let entry = project_root.join("src/main.tyr");
    if !entry.is_file() {
        println!("Model schema comparison skipped: `src/main.tyr` was not found.");
        return Ok(());
    }
    let program = tyrus_core::module_loader::load_program_for_db_migrations(&entry)
        .map_err(|error| format!("could not load project models: {error}"))?;
    let model_sql = match generate_model_schema(&program.items) {
        Ok(sql) if !sql.trim().is_empty() => sql,
        Ok(_) => {
            println!("No `@model` declarations were found in the project.");
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    let model_snapshot = inspect_model_sql(&model_sql)?;
    let mut snapshot_at_model_version = snapshot.clone();
    snapshot_at_model_version.schema_version = model_snapshot.schema_version;
    let changes = super::super::schema_diff(&snapshot_at_model_version, &model_snapshot);
    if changes.is_empty() {
        println!("Model schema matches the database schema snapshot.");
    } else {
        println!("Differences between Models and the database schema snapshot:");
        for change in changes {
            println!("{change}");
        }
        println!("Review the Model or create a migration for the intended schema change.");
    }
    Ok(())
}

fn inspect_model_sql(sql: &str) -> Result<tyrus_core::db::DbSchemaSnapshot, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not initialize model schema comparison".to_string())?;
    runtime.block_on(async {
        let options = SqliteConnectOptions::new()
            .in_memory(true)
            .create_if_missing(true);
        let mut connection = SqliteConnection::connect_with(&options)
            .await
            .map_err(|_| "could not initialize an in-memory SQLite schema".to_string())?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql.to_string()))
            .execute(&mut connection)
            .await
            .map_err(|_| "could not build the schema declared by project Models".to_string())?;
        super::super::inspect_connection(&mut connection, "model-schema.sqlite".to_string()).await
    })
}

fn sqlite_timestamps_trigger_name(table: &str) -> String {
    format!(
        "_tyrus_timestamps_{}_updated_at",
        table.to_ascii_lowercase()
    )
}

fn sqlite_timestamps_trigger_sql(
    model: &tyrus_core::ast::DbModelMetadata,
) -> Result<String, String> {
    let updated_column = model
        .fields
        .iter()
        .find(|field| field.field == "updatedAt")
        .map(|field| field.column.as_str())
        .ok_or_else(|| format!("model `{}` has no `updatedAt` field metadata", model.table))?;
    let trigger_name = sqlite_timestamps_trigger_name(&model.table);
    let primary_key = model
        .fields
        .iter()
        .filter(|field| field.id)
        .map(|field| {
            format!(
                "{} = NEW.{}",
                quote_identifier(&field.column),
                quote_identifier(&field.column)
            )
        })
        .collect::<Vec<_>>();
    if primary_key.is_empty() {
        return Err(format!(
            "model `{}` uses `@timestamps` without an `@id` field",
            model.table
        ));
    }
    Ok(format!(
        "CREATE TRIGGER {} AFTER UPDATE ON {}\nFOR EACH ROW WHEN NEW.{} = OLD.{} AND CURRENT_TIMESTAMP <> OLD.{}\nBEGIN\n    UPDATE {} SET {} = CURRENT_TIMESTAMP WHERE {};\nEND;",
        quote_identifier(&trigger_name),
        quote_identifier(&model.table),
        quote_identifier(updated_column),
        quote_identifier(updated_column),
        quote_identifier(updated_column),
        quote_identifier(&model.table),
        quote_identifier(updated_column),
        primary_key.join(" AND ")
    ))
}

fn sqlite_type(ty: &tyrus_core::ast::Type) -> Option<&'static str> {
    use tyrus_core::ast::Type;
    Some(match ty {
        Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::Isize
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::Bool => "INTEGER",
        Type::F32 | Type::F64 | Type::F128 => "REAL",
        Type::Decimal => "NUMERIC",
        Type::Str | Type::Char => "TEXT",
        Type::Named(name) if name == "Instant" => "TEXT",
        Type::Array(element) if **element == Type::U8 => "BLOB",
        _ => return None,
    })
}

fn validate_model_column_names(
    model_name: &str,
    model: &tyrus_core::ast::DbModelMetadata,
) -> Result<(), String> {
    let mut columns = Vec::new();
    for field in &model.fields {
        if columns
            .iter()
            .any(|column: &&str| sqlite_identifier_eq(column, &field.column))
        {
            return Err(format!(
                "model `{model_name}` maps multiple fields to column `{}`",
                field.column
            ));
        }
        columns.push(field.column.as_str());
    }
    Ok(())
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn sqlite_identifier_eq(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn sqlite_identifier_list_eq(left: &[String], right: &[String]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| sqlite_identifier_eq(left, right))
}

fn sqlite_unique_keys_eq(left: &[Vec<String>], right: &[Vec<String>]) -> bool {
    left.len() == right.len()
        && left.iter().all(|key| {
            right.iter().any(|candidate| {
                key.len() == candidate.len()
                    && key.iter().all(|column| {
                        candidate
                            .iter()
                            .any(|candidate| sqlite_identifier_eq(column, candidate))
                    })
            })
        })
}

fn lookup_sqlite_identifier<'a>(
    entries: &'a BTreeMap<String, Vec<String>>,
    identifier: &str,
) -> Option<&'a Vec<String>> {
    entries
        .iter()
        .find(|(name, _)| sqlite_identifier_eq(name, identifier))
        .map(|(_, value)| value)
}

fn trigger_references_removed_column(trigger_sql: &str, removed_columns: &[&str]) -> bool {
    let single_line = trigger_sql.replace(';', " ");
    let Ok(tokens) = tyrus_core::db::tokenize_sql(&single_line) else {
        return true;
    };
    let matches_removed = |name: &str| {
        removed_columns
            .iter()
            .any(|column| column.eq_ignore_ascii_case(name))
    };
    if tokens.windows(3).any(|window| {
        matches!(window[0].kind, tyrus_core::db::SqlTokenKind::Word)
            && (window[0].text.eq_ignore_ascii_case("OLD")
                || window[0].text.eq_ignore_ascii_case("NEW"))
            && window[1].kind == tyrus_core::db::SqlTokenKind::Dot
            && window[2].kind == tyrus_core::db::SqlTokenKind::Word
            && matches_removed(&window[2].text)
    }) {
        return true;
    }
    for index in 0..tokens.len().saturating_sub(2) {
        if !tokens[index].text.eq_ignore_ascii_case("UPDATE")
            || !tokens[index + 1].text.eq_ignore_ascii_case("OF")
        {
            continue;
        }
        for token in &tokens[index + 2..] {
            if token.text.eq_ignore_ascii_case("ON") {
                break;
            }
            if token.kind == tyrus_core::db::SqlTokenKind::Word && matches_removed(&token.text) {
                return true;
            }
        }
    }
    false
}

fn sqlite_default(kind: &tyrus_core::ast::ExprKind) -> String {
    use tyrus_core::ast::ExprKind;
    match kind {
        ExprKind::StrLit(value) => format!("'{}'", value.replace('\'', "''")),
        ExprKind::IntLit(value) => value.to_string(),
        ExprKind::UIntLit(value) => value.to_string(),
        ExprKind::FloatLit(value) => value.text().to_string(),
        ExprKind::DecimalLit(value) => value.text().to_string(),
        ExprKind::BoolLit(value) => if *value { "1" } else { "0" }.to_string(),
        _ => unreachable!("model defaults are validated as literals"),
    }
}

#[cfg(test)]
mod model_migration_tests {
    use super::super::{NEXT_ARTIFACT_TEMP, write_new_file_atomically};
    use super::generate_model_schema;
    use std::sync::atomic::Ordering;

    fn parse_items(source: &str) -> Vec<tyrus_core::ast::Item> {
        tyrus_core::parser::Parser::from_source(source)
            .expect("source should lex")
            .parse_program()
            .expect("source should parse")
            .items
    }

    #[test]
    fn generates_sqlite_tables_from_model_metadata() {
        let items = parse_items(
            r#"@model("users")
export interface User {
    @id
    id: i64;
    @unique
    email: string;
    @default("Anonymous")
    displayName: string;
    @foreignKey("teams", "id")
    teamId: i64?;
}"#,
        );
        let sql = generate_model_schema(&items).expect("model schema should generate");
        assert!(sql.contains("CREATE TABLE \"users\""));
        assert!(sql.contains("\"id\" INTEGER NOT NULL"));
        assert!(sql.contains("PRIMARY KEY (\"id\")"));
        assert!(sql.contains("\"email\" TEXT UNIQUE NOT NULL"));
        assert!(sql.contains("\"displayName\" TEXT DEFAULT 'Anonymous' NOT NULL"));
        assert!(sql.contains("FOREIGN KEY (\"teamId\") REFERENCES \"teams\" (\"id\")"));
    }

    #[test]
    fn generates_native_mysql_and_postgresql_id_strategy_ddl() {
        use super::generate_server_model_schema;
        use tyrus_core::db::DatabaseBackend;

        let mysql_models = parse_items(
            r#"@model("auto_users") interface AutoUser { @id id: i64; }
@model("uuid_users") interface UuidUser { @id({ strategy: "uuid" }) id: Uuid; }
@model("short_users") interface ShortUser { @id({ strategy: "uuid_short" }) id: u64; }"#,
        );
        let mysql_sql = generate_server_model_schema(&mysql_models, &DatabaseBackend::Mysql)
            .expect("native MySQL ID DDL should be generated");
        assert!(mysql_sql.contains("`id` BIGINT AUTO_INCREMENT NOT NULL"));
        assert!(
            mysql_sql.contains("`createdAt` DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3) NOT NULL")
        );
        assert!(
            mysql_sql.contains("`updatedAt` DATETIME(3) DEFAULT CURRENT_TIMESTAMP(3) NOT NULL")
        );
        assert!(mysql_sql.contains("`id` CHAR(36) DEFAULT (UUID()) NOT NULL"));
        assert!(mysql_sql.contains("`id` BIGINT UNSIGNED DEFAULT (UUID_SHORT()) NOT NULL"));

        let postgresql_models = parse_items(
            r#"@model("identity_users") interface IdentityUser { @id id: i64; }
@model("uuid_users") interface UuidUser { @id({ strategy: "uuid" }) id: Uuid; }
@model("uuid7_users") interface Uuid7User { @id({ strategy: "uuid7" }) id: Uuid7; }"#,
        );
        let postgresql_sql =
            generate_server_model_schema(&postgresql_models, &DatabaseBackend::Postgresql)
                .expect("native PostgreSQL ID DDL should be generated");
        assert!(postgresql_sql.contains("\"id\" BIGINT GENERATED BY DEFAULT AS IDENTITY NOT NULL"));
        assert!(postgresql_sql.contains("\"id\" UUID DEFAULT gen_random_uuid() NOT NULL"));
        assert!(postgresql_sql.contains("\"id\" UUID DEFAULT uuidv7() NOT NULL"));
    }

    #[test]
    fn generates_blob_columns_for_byte_arrays() {
        let items = parse_items(
            r#"@model("assets")
export interface Asset {
    @id
    id: i64;
    payload: u8[];
}"#,
        );
        let sql = generate_model_schema(&items).expect("model schema should generate");
        assert!(sql.contains("\"payload\" BLOB NOT NULL"));
    }

    #[test]
    fn rejects_empty_and_duplicate_model_tables() {
        let empty = parse_items("@model(\"empty\") export interface Empty {}");
        assert!(
            generate_model_schema(&empty)
                .unwrap_err()
                .contains("no fields")
        );
        let duplicate = parse_items(
            "@model(\"same\") interface First { @id id: i64; } @model(\"same\") interface Second { @id id: i64; }",
        );
        assert!(
            generate_model_schema(&duplicate)
                .unwrap_err()
                .contains("multiple models")
        );
    }

    #[test]
    fn rejects_integer_types_that_exceed_sqlite_integer_range() {
        let items = parse_items("@model(\"counters\") interface Counter { value: u64; }");
        assert!(
            generate_model_schema(&items)
                .unwrap_err()
                .contains("cannot be represented")
        );
    }

    #[test]
    fn generated_files_are_published_atomically_without_overwriting() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-db-atomic-write-{}-{}",
            std::process::id(),
            NEXT_ARTIFACT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("temporary test directory should be created");
        let path = root.join("0001_initial.sql");

        write_new_file_atomically(&path, b"CREATE TABLE users (id INTEGER);", "migration")
            .expect("migration file should be published");
        assert_eq!(
            std::fs::read(&path).expect("published file should be readable"),
            b"CREATE TABLE users (id INTEGER);"
        );
        assert!(write_new_file_atomically(&path, b"replacement", "migration").is_err());
        assert_eq!(
            std::fs::read(&path).expect("existing file should remain readable"),
            b"CREATE TABLE users (id INTEGER);"
        );
        assert_eq!(
            std::fs::read_dir(&root)
                .expect("temporary test directory should be readable")
                .count(),
            1,
            "temporary files should be removed after publication"
        );
        std::fs::remove_dir_all(root).expect("temporary test directory should be removed");
    }
}
