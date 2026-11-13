use super::migrate;
use std::path::Path;
use tyrus_core::ast::{ExprKind, Item, Type};
use tyrus_core::db::{
    DatabaseBackend, DatabaseConfig, DbColumnSchema, DbForeignKeySchema, DbSchemaSnapshot,
    DbTableSchema,
};

pub(super) fn diff_models(project_root: &Path, snapshot: &DbSchemaSnapshot) -> Result<(), String> {
    let entry = project_root.join("src/main.tyr");
    if !entry.is_file() {
        println!("Model schema comparison skipped: `src/main.tyr` was not found.");
        return Ok(());
    }
    let program = tyrus_core::module_loader::load_program_for_db_migrations(&entry)
        .map_err(|error| format!("could not load project models: {error}"))?;
    if !program.items.iter().any(|item| {
        matches!(
            item,
            Item::InterfaceDecl {
                db_model: Some(_),
                ..
            }
        )
    }) {
        println!("No `@model` declarations were found in the project.");
        return Ok(());
    }
    let model_snapshot = model_snapshot(&program.items, snapshot)?;
    let mut comparable_snapshot = snapshot.clone();
    let mut comparable_models = model_snapshot;
    for table in comparable_snapshot
        .tables
        .iter_mut()
        .chain(&mut comparable_models.tables)
    {
        table.engine = None;
        for column in &mut table.columns {
            column.native_type = None;
        }
    }
    if snapshot.backend == DatabaseBackend::Mysql {
        normalize_mysql_identifiers(&mut comparable_snapshot);
        normalize_mysql_identifiers(&mut comparable_models);
    }
    normalize_comparison_order(&mut comparable_snapshot);
    normalize_comparison_order(&mut comparable_models);
    for table in comparable_snapshot
        .tables
        .iter_mut()
        .chain(&mut comparable_models.tables)
    {
        for foreign_key in &mut table.foreign_keys {
            foreign_key.on_update.clear();
            foreign_key.on_delete.clear();
        }
    }
    let changes = super::schema_diff(&comparable_snapshot, &comparable_models);
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

fn model_snapshot(items: &[Item], current: &DbSchemaSnapshot) -> Result<DbSchemaSnapshot, String> {
    let backend = &current.backend;
    if !matches!(
        backend,
        DatabaseBackend::Postgresql | DatabaseBackend::Mysql
    ) {
        return Err("server Model comparison requires PostgreSQL or MySQL".to_string());
    }
    migrate::validate_initial_model_foreign_keys(items, false)?;
    migrate::generate_server_model_schema(items, backend)?;
    let mut tables = Vec::new();
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
        let mut columns = Vec::new();
        let mut primary_key = Vec::new();
        let mut unique = Vec::new();
        let mut foreign_keys = Vec::new();
        for (ordinal, metadata) in model.fields.iter().enumerate() {
            let field = fields
                .iter()
                .find(|field| field.name == metadata.field)
                .ok_or_else(|| format!("model `{name}` field metadata is inconsistent"))?;
            let (base_type, nullable) = match &field.ty {
                Type::Optional(inner) => (inner.as_ref(), true),
                ty => (ty, false),
            };
            let sql_type = migrate::server_sql_type(base_type, backend).ok_or_else(|| {
                format!(
                    "model `{name}` field `{}` has no supported server type mapping",
                    field.name
                )
            })?;
            let logical_type = match sql_type {
                "SMALLINT" | "INTEGER" | "BIGINT" => "i64",
                "BOOLEAN" => "boolean",
                "REAL" | "DOUBLE PRECISION" | "DOUBLE" => "f64",
                "TEXT" | "LONGTEXT" => "string",
                "TIMESTAMPTZ" | "DATETIME(3)" => "instant",
                "BYTEA" | "BLOB" => "bytes",
                _ => return Err(format!("model `{name}` uses an unsupported server type")),
            };
            let default = metadata
                .default_value
                .as_ref()
                .map(|value| {
                    let literal = migrate::server_default(value.kind(), backend);
                    if *backend == DatabaseBackend::Postgresql
                        && matches!(base_type, Type::Str | Type::Char)
                        && matches!(value.kind(), ExprKind::StrLit(_))
                    {
                        format!("{literal}::text")
                    } else if matches!(base_type, Type::Bool) {
                        literal.to_ascii_lowercase()
                    } else {
                        literal
                    }
                })
                .or_else(|| {
                    (model.timestamps
                        && matches!(metadata.field.as_str(), "createdAt" | "updatedAt"))
                    .then(|| "CURRENT_TIMESTAMP".to_string())
                });
            columns.push(DbColumnSchema {
                name: metadata.column.clone(),
                logical_type: logical_type.to_string(),
                nullable,
                ordinal: u32::try_from(ordinal)
                    .map_err(|_| "model field ordinal is out of range".to_string())?,
                default,
                native_type: None,
            });
            if metadata.id {
                primary_key.push(metadata.column.clone());
            }
            if metadata.unique {
                unique.push(vec![metadata.column.clone()]);
            }
            if let Some((referenced_table, referenced_column)) = &metadata.foreign_key {
                foreign_keys.push(DbForeignKeySchema {
                    columns: vec![metadata.column.clone()],
                    referenced_table: referenced_table.clone(),
                    referenced_columns: vec![referenced_column.clone()],
                    on_update: if *backend == DatabaseBackend::Mysql {
                        "RESTRICT".to_string()
                    } else {
                        "NO ACTION".to_string()
                    },
                    on_delete: if *backend == DatabaseBackend::Mysql {
                        "RESTRICT".to_string()
                    } else {
                        "NO ACTION".to_string()
                    },
                });
            }
        }
        let existing = current.tables.iter().find(|table| {
            identifier_eq(
                &table.name,
                &model.table,
                *backend == DatabaseBackend::Mysql,
            )
        });
        let mut indexes = existing.map_or_else(Vec::new, |table| table.indexes.clone());
        let mut triggers = existing.map_or_else(Vec::new, |table| table.triggers.clone());
        indexes.sort_by(|left, right| left.name.cmp(&right.name));
        triggers.sort_by(|left, right| left.name.cmp(&right.name));
        unique.sort();
        foreign_keys.sort_by(|left, right| {
            (
                &left.columns,
                &left.referenced_table,
                &left.referenced_columns,
            )
                .cmp(&(
                    &right.columns,
                    &right.referenced_table,
                    &right.referenced_columns,
                ))
        });
        tables.push(DbTableSchema {
            name: model.table.clone(),
            engine: None,
            columns,
            primary_key,
            unique,
            foreign_keys,
            indexes,
            triggers,
        });
    }
    tables.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(DbSchemaSnapshot::new_for_backend(
        backend.clone(),
        current.source.clone(),
        current.schema_version,
        tables,
    ))
}

fn normalize_mysql_identifiers(snapshot: &mut DbSchemaSnapshot) {
    for table in &mut snapshot.tables {
        table.name.make_ascii_lowercase();
        for column in &mut table.columns {
            column.name.make_ascii_lowercase();
        }
        for key in &mut table.primary_key {
            key.make_ascii_lowercase();
        }
        for key in &mut table.unique {
            for column in key {
                column.make_ascii_lowercase();
            }
        }
        for foreign_key in &mut table.foreign_keys {
            for column in &mut foreign_key.columns {
                column.make_ascii_lowercase();
            }
            foreign_key.referenced_table.make_ascii_lowercase();
            for column in &mut foreign_key.referenced_columns {
                column.make_ascii_lowercase();
            }
        }
        for index in &mut table.indexes {
            index.name.make_ascii_lowercase();
            for column in &mut index.columns {
                column.make_ascii_lowercase();
            }
        }
        for trigger in &mut table.triggers {
            trigger.name.make_ascii_lowercase();
        }
    }
}

fn normalize_comparison_order(snapshot: &mut DbSchemaSnapshot) {
    for table in &mut snapshot.tables {
        table.unique.sort();
        table.foreign_keys.sort_by(|left, right| {
            (
                &left.columns,
                &left.referenced_table,
                &left.referenced_columns,
            )
                .cmp(&(
                    &right.columns,
                    &right.referenced_table,
                    &right.referenced_columns,
                ))
        });
    }
}

fn is_lossless_server_type_widening(
    existing_native_type: Option<&str>,
    desired_definition: &str,
    backend: &DatabaseBackend,
) -> bool {
    let Some(existing) = existing_native_type.and_then(|value| numeric_type_rank(value, backend))
    else {
        return false;
    };
    let Some(desired) = numeric_type_rank(desired_definition, backend) else {
        return false;
    };
    existing.0 == desired.0 && existing.1 < desired.1
}

fn server_native_type_matches(
    existing_native_type: &str,
    desired_definition: &str,
    backend: &DatabaseBackend,
) -> bool {
    fn canonical(value: &str, backend: &DatabaseBackend) -> Option<&'static str> {
        let normalized = value.trim().to_ascii_lowercase();
        let words = normalized.split_whitespace().collect::<Vec<_>>();
        let first = words.first()?.split('(').next()?;
        Some(match (backend, first, words.get(1).copied()) {
            (DatabaseBackend::Postgresql, "smallint" | "int2", _)
            | (DatabaseBackend::Mysql, "smallint", _) => "smallint",
            (DatabaseBackend::Postgresql, "integer" | "int4", _)
            | (DatabaseBackend::Mysql, "int" | "integer", _) => "integer",
            (DatabaseBackend::Postgresql, "bigint" | "int8", _)
            | (DatabaseBackend::Mysql, "bigint", _) => "bigint",
            (DatabaseBackend::Postgresql, "boolean" | "bool", _)
            | (DatabaseBackend::Mysql, "tinyint", _) => "boolean",
            (DatabaseBackend::Postgresql, "real" | "float4", _)
            | (DatabaseBackend::Mysql, "real" | "float", _) => "real",
            (DatabaseBackend::Postgresql, "double", Some("precision"))
            | (DatabaseBackend::Postgresql, "float8", _)
            | (DatabaseBackend::Mysql, "double", _) => "double",
            (DatabaseBackend::Postgresql, "text", _) | (DatabaseBackend::Mysql, "longtext", _) => {
                "text"
            }
            (DatabaseBackend::Postgresql, "bytea", _)
            | (DatabaseBackend::Mysql, "blob" | "longblob" | "mediumblob", _) => "bytes",
            _ => return None,
        })
    }
    let desired_type = desired_definition
        .split_whitespace()
        .take_while(|word| {
            !matches!(
                word.to_ascii_uppercase().as_str(),
                "NOT" | "NULL" | "DEFAULT" | "UNIQUE" | "GENERATED" | "AUTO_INCREMENT"
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let existing = canonical(existing_native_type, backend);
    existing.is_some() && existing == canonical(&desired_type, backend)
}

fn numeric_type_rank(value: &str, backend: &DatabaseBackend) -> Option<(&'static str, u8)> {
    let normalized = value.trim().to_ascii_lowercase();
    let name = normalized.split([' ', '(']).next()?;
    let rank = match (backend, name) {
        (DatabaseBackend::Postgresql, "int2" | "smallint")
        | (DatabaseBackend::Mysql, "smallint") => ("integer", 1),
        (DatabaseBackend::Postgresql, "int4" | "integer")
        | (DatabaseBackend::Mysql, "int" | "integer") => ("integer", 2),
        (DatabaseBackend::Postgresql, "int8" | "bigint") | (DatabaseBackend::Mysql, "bigint") => {
            ("integer", 3)
        }
        (DatabaseBackend::Postgresql, "float4" | "real")
        | (DatabaseBackend::Mysql, "float" | "real") => ("float", 1),
        (DatabaseBackend::Postgresql, "float8" | "double") => ("float", 2),
        (DatabaseBackend::Mysql, "double") => ("float", 2),
        _ => return None,
    };
    Some(rank)
}

fn server_sql_type_for_definition(definition: &str) -> Result<&'static str, String> {
    let normalized = definition.trim().to_ascii_uppercase();
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    match words.as_slice() {
        ["SMALLINT", ..] => Ok("SMALLINT"),
        ["INTEGER", ..] => Ok("INTEGER"),
        ["BIGINT", ..] => Ok("BIGINT"),
        ["REAL", ..] => Ok("REAL"),
        ["DOUBLE", "PRECISION", ..] => Ok("DOUBLE PRECISION"),
        _ => Err("could not determine the generated PostgreSQL column type".to_string()),
    }
}

pub(super) fn generate(
    project_root: &Path,
    name: &str,
    config: &DatabaseConfig,
    items: &[Item],
) -> Result<(), String> {
    migrate::validate_name(name)?;
    let snapshot_path = config.resolve_schema_path(project_root)?;
    let expected = DbSchemaSnapshot::load(&snapshot_path).map_err(|_| {
        "database schema snapshot is missing or invalid; run `tyrus db introspect` first"
            .to_string()
    })?;
    let actual = super::inspect_configured(project_root, config)?;
    if expected != actual {
        return Err("database schema drift detected; run `tyrus db diff` and `tyrus db introspect` before generating a model migration".to_string());
    }

    let desired_sql = migrate::generate_server_model_schema(items, &config.backend)?;
    let statements = desired_sql
        .split("\n\n")
        .filter(|statement| !statement.is_empty())
        .collect::<Vec<_>>();
    let mut generated = Vec::new();
    for item in items {
        let Item::InterfaceDecl {
            name: model_name,
            fields,
            db_model: Some(model),
            ..
        } = item
        else {
            continue;
        };
        let quote = migrate::quote_identifier_for_backend(&model.table, &config.backend);
        let prefix = format!("CREATE TABLE {quote} (");
        let statement = statements
            .iter()
            .find(|statement| statement.starts_with(&prefix))
            .copied()
            .ok_or_else(|| format!("could not match generated schema for model `{model_name}`"))?;
        generated.push((model_name.as_str(), fields, model, statement));
    }
    if statements.len() != generated.len() {
        return Err("generated model schema contains unmatched tables".to_string());
    }

    let mut changes = Vec::new();
    for (model_name, fields, model, statement) in generated {
        let existing = actual.tables.iter().find(|table| {
            identifier_eq(
                &table.name,
                &model.table,
                config.backend == DatabaseBackend::Mysql,
            )
        });
        let Some(table) = existing else {
            changes.push(statement.to_string());
            continue;
        };
        let additions = compare_existing_model(
            model_name,
            fields,
            model,
            table,
            &actual,
            statement,
            &config.backend,
        )?;
        changes.extend(additions);
    }
    if changes.is_empty() {
        return Err(
            "Models match the live server schema; no incremental migration is needed".to_string(),
        );
    }
    migrate::create_with_sql(project_root, name, &(changes.join("\n\n") + "\n"))
}

fn compare_existing_model(
    model_name: &str,
    fields: &[tyrus_core::ast::Field],
    model: &tyrus_core::ast::DbModelMetadata,
    table: &DbTableSchema,
    schema: &DbSchemaSnapshot,
    statement: &str,
    backend: &DatabaseBackend,
) -> Result<Vec<String>, String> {
    if model.timestamps {
        return Err(format!(
            "incremental generation for model `{model_name}` with `@timestamps` requires a manually reviewed migration"
        ));
    }
    let mut definitions = Vec::new();
    for line in statement.lines().skip(1) {
        let definition = line.trim().trim_end_matches(',');
        if definition.starts_with("PRIMARY KEY ") || definition.starts_with("FOREIGN KEY ") {
            continue;
        }
        if !definition.is_empty() {
            definitions.push(definition);
        }
    }
    let quote = |identifier: &str| migrate::quote_identifier_for_backend(identifier, backend);
    let mut additions = Vec::new();
    let mut model_columns = Vec::new();
    for definition in definitions {
        let Some((column_name, sql_definition)) = split_column_definition(definition, backend)
        else {
            return Err(format!(
                "could not safely inspect generated columns for model `{model_name}`"
            ));
        };
        model_columns.push(column_name.clone());
        let field = model
            .fields
            .iter()
            .find(|field| {
                identifier_eq(
                    &field.column,
                    &column_name,
                    *backend == DatabaseBackend::Mysql,
                )
            })
            .ok_or_else(|| {
                format!("generated column `{column_name}` is not declared by model `{model_name}`")
            })?;
        if let Some(existing) = table.columns.iter().find(|column| {
            identifier_eq(
                &column.name,
                &column_name,
                *backend == DatabaseBackend::Mysql,
            )
        }) {
            let declaration = model_field_type(model_name, field, fields, backend)?;
            let expected_nullable = fields
                .iter()
                .find(|candidate| candidate.name == field.field)
                .is_some_and(|candidate| matches!(candidate.ty, Type::Optional(_)));
            let type_widening = is_lossless_server_type_widening(
                existing.native_type.as_deref(),
                &sql_definition,
                backend,
            );
            let native_type_changed = existing.native_type.as_deref().is_some_and(|native| {
                !server_native_type_matches(native, &sql_definition, backend)
            });
            if ((declaration != existing.logical_type || native_type_changed) && !type_widening)
                || (existing.nullable != expected_nullable && !expected_nullable)
            {
                return Err(format!(
                    "model `{model_name}` narrows or changes the type or nullability of existing column `{}`; author and review this server migration manually",
                    existing.name
                ));
            }
            if type_widening {
                let has_key_dependency = field.id
                    || field.unique
                    || has_unique_column(table, &column_name, backend)
                    || table.foreign_keys.iter().any(|key| {
                        key.columns.iter().any(|column| {
                            identifier_eq(column, &column_name, *backend == DatabaseBackend::Mysql)
                        })
                    })
                    || table.indexes.iter().any(|index| {
                        index.columns.iter().any(|column| {
                            identifier_eq(column, &column_name, *backend == DatabaseBackend::Mysql)
                        })
                    })
                    || schema.tables.iter().any(|source| {
                        source.foreign_keys.iter().any(|key| {
                            identifier_eq(
                                &key.referenced_table,
                                &table.name,
                                *backend == DatabaseBackend::Mysql,
                            ) && key.referenced_columns.iter().any(|column| {
                                identifier_eq(
                                    column,
                                    &column_name,
                                    *backend == DatabaseBackend::Mysql,
                                )
                            })
                        })
                    });
                if has_key_dependency {
                    return Err(format!(
                        "model `{model_name}` widens key column `{}.{}`; review the dependent constraints and author this server migration manually",
                        table.name, existing.name
                    ));
                }
                match backend {
                    DatabaseBackend::Postgresql => {
                        let target_type = server_sql_type_for_definition(&sql_definition)?;
                        additions.push(format!(
                            "ALTER TABLE {} ALTER COLUMN {} TYPE {} USING {}::{};",
                            quote(&table.name),
                            quote(&existing.name),
                            target_type,
                            quote(&existing.name),
                            target_type
                        ));
                    }
                    DatabaseBackend::Mysql => additions.push(format!(
                        "ALTER TABLE {} MODIFY COLUMN {} {};",
                        quote(&table.name),
                        quote(&existing.name),
                        sql_definition
                    )),
                    _ => unreachable!("server Model migrations only support server backends"),
                }
            }
            if existing.nullable && !expected_nullable {
                unreachable!("nullability tightening was rejected above");
            } else if !existing.nullable && expected_nullable {
                if *backend == DatabaseBackend::Mysql && existing.native_type.is_none() {
                    return Err(format!(
                        "snapshot lacks the native type of `{}.{}`; run `tyrus db introspect` before generating this MySQL migration",
                        table.name, existing.name
                    ));
                }
                match backend {
                    DatabaseBackend::Postgresql => additions.push(format!(
                        "ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL;",
                        quote(&table.name),
                        quote(&existing.name)
                    )),
                    DatabaseBackend::Mysql => {
                        let nullable_definition = sql_definition
                            .replace(" NOT NULL", " NULL")
                            .replace(" not null", " NULL");
                        additions.push(format!(
                            "ALTER TABLE {} MODIFY COLUMN {} {};",
                            quote(&table.name),
                            quote(&existing.name),
                            nullable_definition
                        ));
                    }
                    _ => unreachable!("server Model migrations only support server backends"),
                }
            }
            if field.id
                != table
                    .primary_key
                    .iter()
                    .any(|key| identifier_eq(key, &column_name, *backend == DatabaseBackend::Mysql))
            {
                return Err(format!(
                    "model `{model_name}` changes the primary key of table `{}`; author and review this server migration manually",
                    table.name
                ));
            }
            let has_single_column_unique =
                field.id || has_unique_column(table, &column_name, backend);
            if field.unique && !has_single_column_unique {
                additions.push(format!(
                    "ALTER TABLE {} ADD UNIQUE ({});",
                    quote(&table.name),
                    quote(&column_name)
                ));
            } else if !field.unique && has_single_column_unique && !field.id {
                return Err(format!(
                    "model `{model_name}` removes an existing unique constraint from table `{}`; author and review this server migration manually",
                    table.name
                ));
            }
            if let Some((target_table, target_column)) = &field.foreign_key {
                if !expected_nullable {
                    return Err(format!(
                        "adding foreign-key metadata to existing required column `{}.{}` requires a manually reviewed server migration",
                        table.name, field.column
                    ));
                }
                let already_matches = table.foreign_keys.iter().any(|key| {
                    key.columns.len() == 1
                        && key.referenced_columns.len() == 1
                        && identifier_eq(
                            &key.columns[0],
                            &field.column,
                            *backend == DatabaseBackend::Mysql,
                        )
                        && identifier_eq(
                            &key.referenced_table,
                            target_table,
                            *backend == DatabaseBackend::Mysql,
                        )
                        && identifier_eq(
                            &key.referenced_columns[0],
                            target_column,
                            *backend == DatabaseBackend::Mysql,
                        )
                });
                if !already_matches {
                    if table.foreign_keys.iter().any(|key| {
                        key.columns.iter().any(|column| {
                            identifier_eq(column, &field.column, *backend == DatabaseBackend::Mysql)
                        })
                    }) {
                        return Err(format!(
                            "model `{model_name}` changes an existing foreign key on `{}.{}`; author and review this server migration manually",
                            table.name, field.column
                        ));
                    }
                    let declaration = fields
                        .iter()
                        .find(|candidate| candidate.name == field.field)
                        .ok_or_else(|| {
                            format!("model `{model_name}` field metadata is inconsistent")
                        })?;
                    let base_type = match &declaration.ty {
                        Type::Optional(inner) => inner.as_ref(),
                        ty => ty,
                    };
                    let source_type = model_field_type(model_name, field, fields, backend)?;
                    let source_native_type = migrate::server_sql_type(base_type, backend)
                        .ok_or_else(|| {
                            format!(
                                "model `{model_name}` foreign-key field has no server type mapping"
                            )
                        })?;
                    validate_existing_foreign_key(
                        model_name,
                        target_table,
                        target_column,
                        &source_type,
                        source_native_type,
                        table,
                        schema,
                        backend,
                    )?;
                    additions.push(format!(
                        "ALTER TABLE {} ADD FOREIGN KEY ({}) REFERENCES {} ({});",
                        quote(&table.name),
                        quote(&field.column),
                        quote(target_table),
                        quote(target_column)
                    ));
                }
            }
            if !field.id || field.default_value.is_some() {
                let desired_default = server_model_default(model_name, field, fields, backend)?;
                match (desired_default, existing.default.as_deref()) {
                    (Some(default), current) if !server_defaults_match(&default, current) => {
                        additions.push(format!(
                            "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};",
                            quote(&table.name),
                            quote(&existing.name),
                            default
                        ));
                    }
                    (None, Some(current)) if is_literal_server_default(current) => {
                        additions.push(format!(
                            "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;",
                            quote(&table.name),
                            quote(&existing.name)
                        ));
                    }
                    (None, Some(_)) => {
                        return Err(format!(
                            "model `{model_name}` removes a non-literal default from `{}.{}`; author and review this server migration manually",
                            table.name, existing.name
                        ));
                    }
                    _ => {}
                }
            }
        } else {
            if field.id {
                return Err(format!(
                    "adding a primary-key column `{}` to existing table `{}` requires a manually reviewed server migration",
                    field.column, table.name
                ));
            }
            let nullable = fields
                .iter()
                .find(|candidate| candidate.name == field.field)
                .is_some_and(|candidate| matches!(candidate.ty, Type::Optional(_)));
            if !nullable && field.default_value.is_none() {
                return Err(format!(
                    "new required column `{}.{}` needs a literal default before it can be added safely",
                    table.name, field.column
                ));
            }
            if field.unique && (!nullable || field.default_value.is_some()) {
                return Err(format!(
                    "adding unique column `{}.{}` requires an optional field without a default to avoid collisions with existing rows",
                    table.name, field.column
                ));
            }
            if let Some((target_table, target_column)) = &field.foreign_key {
                if !matches!(
                    backend,
                    DatabaseBackend::Postgresql | DatabaseBackend::Mysql
                ) || !nullable
                    || field.default_value.is_some()
                {
                    return Err(format!(
                        "adding foreign-key column `{}.{}` requires an optional PostgreSQL/MySQL field without a default; author and review this server migration manually",
                        table.name, field.column
                    ));
                }
                let source_type = model_field_type(model_name, field, fields, backend)?;
                let source_field = fields
                    .iter()
                    .find(|candidate| candidate.name == field.field)
                    .ok_or_else(|| {
                        format!("model `{model_name}` field metadata is inconsistent")
                    })?;
                let source_base_type = match &source_field.ty {
                    Type::Optional(inner) => inner.as_ref(),
                    ty => ty,
                };
                let source_native_type = migrate::server_sql_type(source_base_type, backend)
                    .ok_or_else(|| {
                        format!("model `{model_name}` foreign-key field has no server type mapping")
                    })?;
                validate_existing_foreign_key(
                    model_name,
                    target_table,
                    target_column,
                    &source_type,
                    source_native_type,
                    table,
                    schema,
                    backend,
                )?;
            }
            let sql = format!(
                "ALTER TABLE {} ADD COLUMN {};",
                quote(&table.name),
                sql_definition
            );
            additions.push(sql);
            if let Some((target_table, target_column)) = &field.foreign_key {
                additions.push(format!(
                    "ALTER TABLE {} ADD FOREIGN KEY ({}) REFERENCES {} ({});",
                    quote(&table.name),
                    quote(&field.column),
                    quote(target_table),
                    quote(target_column)
                ));
            }
        }
    }
    for existing in &table.columns {
        if !model_columns
            .iter()
            .any(|column| identifier_eq(column, &existing.name, *backend == DatabaseBackend::Mysql))
        {
            return Err(format!(
                "model `{model_name}` removes existing column `{}.{}`; author and review this server migration manually",
                table.name, existing.name
            ));
        }
    }
    let model_primary_key = model
        .fields
        .iter()
        .filter(|field| field.id)
        .map(|field| field.column.as_str())
        .collect::<Vec<_>>();
    let primary_key_matches = model_primary_key.len() == table.primary_key.len()
        && model_primary_key
            .iter()
            .zip(&table.primary_key)
            .all(|(expected, actual)| {
                identifier_eq(expected, actual, *backend == DatabaseBackend::Mysql)
            });
    if !primary_key_matches
        || table.foreign_keys.iter().any(|key| {
            !model.fields.iter().any(|field| {
                field
                    .foreign_key
                    .as_ref()
                    .is_some_and(|(target_table, target_column)| {
                        key.columns.len() == 1
                            && key.referenced_columns.len() == 1
                            && identifier_eq(
                                &key.columns[0],
                                &field.column,
                                *backend == DatabaseBackend::Mysql,
                            )
                            && identifier_eq(
                                &key.referenced_table,
                                target_table,
                                *backend == DatabaseBackend::Mysql,
                            )
                            && identifier_eq(
                                &key.referenced_columns[0],
                                target_column,
                                *backend == DatabaseBackend::Mysql,
                            )
                    })
            })
        })
    {
        return Err(format!(
            "model `{model_name}` changes key relationships on table `{}`; author and review this server migration manually",
            table.name
        ));
    }
    Ok(additions)
}

fn server_model_default(
    model_name: &str,
    field: &tyrus_core::ast::DbModelFieldMetadata,
    fields: &[tyrus_core::ast::Field],
    backend: &DatabaseBackend,
) -> Result<Option<String>, String> {
    let Some(value) = &field.default_value else {
        return Ok(None);
    };
    let declaration = fields
        .iter()
        .find(|candidate| candidate.name == field.field)
        .ok_or_else(|| format!("model `{model_name}` field metadata is inconsistent"))?;
    let base_type = match &declaration.ty {
        Type::Optional(inner) => inner.as_ref(),
        ty => ty,
    };
    let mut default = migrate::server_default(value.kind(), backend);
    if *backend == DatabaseBackend::Postgresql
        && matches!(base_type, Type::Str | Type::Char)
        && matches!(value.kind(), ExprKind::StrLit(_))
    {
        default.push_str("::text");
    }
    Ok(Some(default))
}

fn has_unique_column(table: &DbTableSchema, column: &str, backend: &DatabaseBackend) -> bool {
    let insensitive = *backend == DatabaseBackend::Mysql;
    table
        .unique
        .iter()
        .any(|key| key.len() == 1 && identifier_eq(&key[0], column, insensitive))
        || table.indexes.iter().any(|index| {
            index.unique
                && !index.partial
                && !index.expression
                && index.columns.len() == 1
                && identifier_eq(&index.columns[0], column, insensitive)
        })
}

fn server_defaults_match(expected: &str, actual: Option<&str>) -> bool {
    actual.is_some_and(|actual| {
        normalize_server_default(expected) == normalize_server_default(actual)
    })
}

fn normalize_server_default(value: &str) -> String {
    let mut normalized = value.trim().to_string();
    while normalized.starts_with('(') && normalized.ends_with(')') {
        normalized = normalized[1..normalized.len() - 1].trim().to_string();
    }
    if let Some((literal, _cast)) = normalized.split_once("::") {
        normalized = literal.trim().to_string();
    }
    if normalized.len() >= 2 && normalized.starts_with('\'') && normalized.ends_with('\'') {
        normalized = normalized[1..normalized.len() - 1].replace("''", "'");
    } else {
        normalized.make_ascii_lowercase();
    }
    normalized
}

fn is_literal_server_default(value: &str) -> bool {
    let mut literal = value.trim();
    if let Some((value, _cast)) = literal.split_once("::") {
        literal = value.trim();
    }
    while literal.starts_with('(') && literal.ends_with(')') {
        literal = literal[1..literal.len() - 1].trim();
    }
    (literal.starts_with('\'') && literal.ends_with('\''))
        || matches!(literal.to_ascii_lowercase().as_str(), "true" | "false")
        || literal.parse::<i128>().is_ok()
        || literal.parse::<f64>().is_ok()
}

fn validate_existing_foreign_key(
    model_name: &str,
    target_table_name: &str,
    target_column_name: &str,
    source_type: &str,
    source_native_type: &str,
    source_table: &DbTableSchema,
    schema: &DbSchemaSnapshot,
    backend: &DatabaseBackend,
) -> Result<(), String> {
    let insensitive = *backend == DatabaseBackend::Mysql;
    let target_table = schema
        .tables
        .iter()
        .find(|table| identifier_eq(&table.name, target_table_name, insensitive))
        .ok_or_else(|| {
            format!(
                "foreign-key target table `{target_table_name}` for model `{model_name}` is not present in the live schema; generate its table first or author SQL manually"
            )
        })?;
    let target_column = target_table
        .columns
        .iter()
        .find(|column| identifier_eq(&column.name, target_column_name, insensitive))
        .ok_or_else(|| {
            format!(
                "foreign-key target column `{}.{target_column_name}` for model `{model_name}` is not present in the live schema",
                target_table.name
            )
        })?;
    let target_is_unique = target_table
        .primary_key
        .iter()
        .any(|column| identifier_eq(column, target_column_name, insensitive))
        || target_table
            .unique
            .iter()
            .any(|key| key.len() == 1 && identifier_eq(&key[0], target_column_name, insensitive));
    let backend_type_matches = if *backend == DatabaseBackend::Mysql {
        source_table
            .engine
            .as_deref()
            .is_some_and(|engine| engine.eq_ignore_ascii_case("innodb"))
            && target_table
                .engine
                .as_deref()
                .is_some_and(|engine| engine.eq_ignore_ascii_case("innodb"))
            && target_column
                .native_type
                .as_deref()
                .is_some_and(|native_type| {
                    mysql_integer_type_matches(source_native_type, native_type)
                })
    } else {
        target_column
            .native_type
            .as_deref()
            .is_some_and(|native_type| postgresql_type_matches(source_native_type, native_type))
    };
    if !target_is_unique || target_column.logical_type != source_type || !backend_type_matches {
        return Err(format!(
            "foreign-key target `{}.{target_column_name}` for model `{model_name}` must be unique, type-compatible, and use InnoDB with complete MySQL type metadata when the backend is MySQL",
            target_table.name
        ));
    }
    Ok(())
}

fn postgresql_type_matches(source_type: &str, target_type: &str) -> bool {
    let expected = match source_type.to_ascii_lowercase().as_str() {
        "smallint" => "int2",
        "integer" => "int4",
        "bigint" => "int8",
        "boolean" => "bool",
        "real" => "float4",
        "double precision" => "float8",
        "text" => "text",
        "bytea" => "bytea",
        _ => return false,
    };
    target_type.eq_ignore_ascii_case(expected)
}

fn mysql_integer_type_matches(source_type: &str, target_type: &str) -> bool {
    fn signature(sql_type: &str) -> Option<(&'static str, bool)> {
        let normalized = sql_type.to_ascii_lowercase();
        let mut words = normalized.split_whitespace();
        let raw_name = words.next()?;
        let name = raw_name.split('(').next()?;
        let unsigned = words.any(|word| word == "unsigned" || word == "zerofill");
        let canonical = match name {
            "smallint" => "smallint",
            "integer" | "int" => "int",
            "bigint" => "bigint",
            _ => return None,
        };
        Some((canonical, unsigned))
    }
    signature(source_type)
        .zip(signature(target_type))
        .is_some_and(|(source, target)| source == target)
}

fn split_column_definition(
    definition: &str,
    backend: &DatabaseBackend,
) -> Option<(String, String)> {
    let (open, close) = if *backend == DatabaseBackend::Mysql {
        ('`', '`')
    } else {
        ('"', '"')
    };
    let rest = definition.strip_prefix(open)?;
    let mut end = None;
    let mut index = 0;
    while index < rest.len() {
        let character = rest[index..].chars().next()?;
        if character == close {
            if rest[index + 1..].starts_with(close) {
                index += 2;
                continue;
            }
            end = Some(index);
            break;
        }
        index += character.len_utf8();
    }
    let end = end?;
    let name = rest[..end].replace(&format!("{close}{close}"), &close.to_string());
    let sql = rest[end + 1..].trim();
    Some((name, sql.to_string()))
}

fn model_field_type(
    model_name: &str,
    field: &tyrus_core::ast::DbModelFieldMetadata,
    fields: &[tyrus_core::ast::Field],
    backend: &DatabaseBackend,
) -> Result<String, String> {
    let declaration = fields
        .iter()
        .find(|candidate| candidate.name == field.field)
        .ok_or_else(|| format!("model `{model_name}` field metadata is inconsistent"))?;
    let base_type = match &declaration.ty {
        Type::Optional(inner) => inner.as_ref(),
        ty => ty,
    };
    let sql_type = migrate::server_sql_type(base_type, backend).ok_or_else(|| {
        format!(
            "model `{model_name}` field `{}` has no server type mapping",
            field.field
        )
    })?;
    Ok(match sql_type {
        "SMALLINT" | "INTEGER" | "BIGINT" => "i64",
        "BOOLEAN" => "boolean",
        "REAL" | "DOUBLE PRECISION" | "DOUBLE" => "f64",
        "TEXT" | "LONGTEXT" => "string",
        "BYTEA" | "BLOB" => "bytes",
        _ => {
            return Err(format!(
                "model `{model_name}` field `{}` has an unsupported server type",
                field.field
            ));
        }
    }
    .to_string())
}

fn identifier_eq(left: &str, right: &str, insensitive: bool) -> bool {
    if insensitive {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}
