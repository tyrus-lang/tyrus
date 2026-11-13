use super::migrate::ServerMigrationBackend;
use sqlx::{Any, Pool, Row};
use std::collections::BTreeMap;
use tyrus_core::db::{
    DatabaseBackend, DbColumnSchema, DbForeignKeySchema, DbIndexSchema, DbSchemaSnapshot,
    DbTableSchema, DbTriggerSchema,
};

pub(super) async fn inspect(
    pool: &Pool<Any>,
    backend: ServerMigrationBackend,
    source: String,
) -> Result<DbSchemaSnapshot, String> {
    let table_sql = table_list_query(backend);
    let mut tables = Vec::new();
    for row in sqlx::query(table_sql)
        .fetch_all(pool)
        .await
        .map_err(|error| {
            let cause = error
                .as_database_error()
                .map(|database_error| database_error.message().to_string())
                .unwrap_or_else(|| error.to_string());
            format!("could not list server database tables: {cause}")
        })?
    {
        let name: String = row
            .try_get("name")
            .map_err(|_| "could not read server database table name".to_string())?;
        let engine = match backend {
            ServerMigrationBackend::Postgresql => None,
            ServerMigrationBackend::Mysql => Some(
                row.try_get::<String, _>("engine")
                    .map_err(|_| "could not read server database table engine".to_string())?,
            ),
        };
        if name.starts_with("_tyrus_") {
            continue;
        }
        tables.push(inspect_table(pool, backend, &name, engine).await?);
    }
    tables.sort_by(|left, right| left.name.cmp(&right.name));
    let backend = match backend {
        ServerMigrationBackend::Postgresql => DatabaseBackend::Postgresql,
        ServerMigrationBackend::Mysql => DatabaseBackend::Mysql,
    };
    Ok(DbSchemaSnapshot::new_for_backend(
        backend, source, 0, tables,
    ))
}

fn table_list_query(backend: ServerMigrationBackend) -> &'static str {
    match backend {
        ServerMigrationBackend::Postgresql => {
            "SELECT table_name::text AS name FROM information_schema.tables WHERE table_schema = current_schema() AND table_type = 'BASE TABLE' ORDER BY table_name"
        }
        ServerMigrationBackend::Mysql => {
            "SELECT table_name AS name, COALESCE(engine, '') AS engine FROM information_schema.tables WHERE table_schema = database() AND table_type = 'BASE TABLE' ORDER BY table_name"
        }
    }
}

fn inspection_queries(
    backend: ServerMigrationBackend,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match backend {
        ServerMigrationBackend::Postgresql => (
            "SELECT column_name::text AS name, data_type AS type, is_nullable AS nullable, ordinal_position AS ordinal, column_default AS default_value, udt_name::text AS native_type FROM information_schema.columns WHERE table_schema = current_schema() AND table_name = $1 ORDER BY ordinal_position",
            "SELECT kcu.column_name::text AS name FROM information_schema.table_constraints tc JOIN information_schema.key_column_usage kcu ON tc.constraint_catalog = kcu.constraint_catalog AND tc.constraint_schema = kcu.constraint_schema AND tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema AND tc.table_name = kcu.table_name WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_schema = current_schema() AND tc.table_name = $1 ORDER BY kcu.ordinal_position",
            "SELECT tc.constraint_name::text AS name, kcu.column_name::text AS column_name, kcu.ordinal_position AS ordinal FROM information_schema.table_constraints tc JOIN information_schema.key_column_usage kcu ON tc.constraint_catalog = kcu.constraint_catalog AND tc.constraint_schema = kcu.constraint_schema AND tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema AND tc.table_name = kcu.table_name WHERE tc.constraint_type = 'UNIQUE' AND tc.table_schema = current_schema() AND tc.table_name = $1 ORDER BY tc.constraint_name, kcu.ordinal_position",
            "SELECT con.conname::text AS name, local_att.attname::text AS column_name, ref_table.relname::text AS referenced_table, ref_att.attname::text AS referenced_column, key.ordinality AS ordinal, CASE con.confupdtype WHEN 'a' THEN 'NO ACTION' WHEN 'r' THEN 'RESTRICT' WHEN 'c' THEN 'CASCADE' WHEN 'n' THEN 'SET NULL' WHEN 'd' THEN 'SET DEFAULT' END AS on_update, CASE con.confdeltype WHEN 'a' THEN 'NO ACTION' WHEN 'r' THEN 'RESTRICT' WHEN 'c' THEN 'CASCADE' WHEN 'n' THEN 'SET NULL' WHEN 'd' THEN 'SET DEFAULT' END AS on_delete FROM pg_constraint con JOIN pg_class local_table ON local_table.oid = con.conrelid JOIN pg_namespace ns ON ns.oid = local_table.relnamespace JOIN pg_class ref_table ON ref_table.oid = con.confrelid CROSS JOIN LATERAL unnest(con.conkey) WITH ORDINALITY AS key(attnum, ordinality) JOIN LATERAL unnest(con.confkey) WITH ORDINALITY AS ref_key(attnum, ordinality) ON ref_key.ordinality = key.ordinality JOIN pg_attribute local_att ON local_att.attrelid = local_table.oid AND local_att.attnum = key.attnum JOIN pg_attribute ref_att ON ref_att.attrelid = ref_table.oid AND ref_att.attnum = ref_key.attnum WHERE con.contype = 'f' AND ns.nspname = current_schema() AND local_table.relname = $1 ORDER BY con.conname, key.ordinality",
            "SELECT ci.relname::text AS name, ix.indisunique AS is_unique, (ix.indpred IS NOT NULL) AS is_partial, (ix.indexprs IS NOT NULL) AS has_expression, COALESCE(a.attname::text, '') AS column_name, key.ordinality AS ordinal FROM pg_class ct JOIN pg_namespace ns ON ns.oid = ct.relnamespace JOIN pg_index ix ON ix.indrelid = ct.oid JOIN pg_class ci ON ci.oid = ix.indexrelid CROSS JOIN LATERAL unnest(ix.indkey::smallint[]) WITH ORDINALITY AS key(attnum, ordinality) LEFT JOIN pg_attribute a ON a.attrelid = ct.oid AND a.attnum = key.attnum WHERE ns.nspname = current_schema() AND ct.relname = $1 AND NOT ix.indisprimary ORDER BY ci.relname, key.ordinality",
            "SELECT t.tgname::text AS name, pg_get_triggerdef(t.oid) AS definition FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = current_schema() AND c.relname = $1 AND NOT t.tgisinternal ORDER BY t.tgname",
        ),
        ServerMigrationBackend::Mysql => (
            "SELECT column_name AS name, data_type AS type, is_nullable AS nullable, ordinal_position AS ordinal, column_default AS default_value, column_type AS native_type FROM information_schema.columns WHERE table_schema = database() AND table_name = ? ORDER BY ordinal_position",
            "SELECT column_name AS name FROM information_schema.key_column_usage WHERE table_schema = database() AND table_name = ? AND constraint_name = 'PRIMARY' ORDER BY ordinal_position",
            "SELECT constraint_name AS name, column_name, ordinal_position AS ordinal FROM information_schema.key_column_usage WHERE table_schema = database() AND table_name = ? AND constraint_name <> 'PRIMARY' AND referenced_table_name IS NULL AND constraint_name IN (SELECT constraint_name FROM information_schema.table_constraints WHERE table_schema = database() AND table_name = ? AND constraint_type = 'UNIQUE') ORDER BY constraint_name, ordinal_position",
            "SELECT kcu.constraint_name AS name, kcu.column_name AS column_name, kcu.referenced_table_name AS referenced_table, kcu.referenced_column_name AS referenced_column, kcu.ordinal_position AS ordinal, rc.update_rule AS on_update, rc.delete_rule AS on_delete FROM information_schema.key_column_usage kcu JOIN information_schema.referential_constraints rc ON rc.constraint_schema = kcu.constraint_schema AND rc.constraint_name = kcu.constraint_name AND rc.table_name = kcu.table_name WHERE kcu.table_schema = database() AND kcu.table_name = ? AND kcu.referenced_table_name IS NOT NULL ORDER BY kcu.constraint_name, kcu.ordinal_position",
            "SELECT index_name AS name, (non_unique = 0) AS is_unique, FALSE AS is_partial, (column_name IS NULL OR sub_part IS NOT NULL) AS has_expression, COALESCE(column_name, '') AS column_name, seq_in_index AS ordinal FROM information_schema.statistics WHERE table_schema = database() AND table_name = ? AND index_name <> 'PRIMARY' ORDER BY index_name, seq_in_index",
            "SELECT trigger_name AS name, CONCAT(action_timing, ' ', event_manipulation, ' ON ', event_object_table, ' FOR EACH ', action_orientation, ' ', action_statement) AS definition FROM information_schema.triggers WHERE trigger_schema = database() AND event_object_table = ? ORDER BY trigger_name",
        ),
    }
}

async fn inspect_table(
    pool: &Pool<Any>,
    backend: ServerMigrationBackend,
    name: &str,
    engine: Option<String>,
) -> Result<DbTableSchema, String> {
    let (columns_sql, key_sql, unique_sql, foreign_sql, index_sql, trigger_sql) =
        inspection_queries(backend);
    let column_rows = query_for_table(columns_sql, backend, name)
        .fetch_all(pool)
        .await
        .map_err(|error| {
            let cause = error
                .as_database_error()
                .map(|database_error| database_error.message().to_string())
                .unwrap_or_else(|| error.to_string());
            format!("could not inspect server database columns: {cause}")
        })?;
    let mut columns = Vec::with_capacity(column_rows.len());
    for row in column_rows {
        let native_type: String = row
            .try_get("native_type")
            .map_err(|_| "could not read server column type".to_string())?;
        let type_name: String = row
            .try_get("type")
            .map_err(|_| "could not read server column type".to_string())?;
        let nullable: String = row
            .try_get("nullable")
            .map_err(|_| "could not read server column nullability".to_string())?;
        let ordinal: i64 = row
            .try_get("ordinal")
            .map_err(|_| "could not read server column ordinal".to_string())?;
        let default: Option<String> = row
            .try_get("default_value")
            .map_err(|_| "could not read server column default".to_string())?;
        columns.push(DbColumnSchema {
            name: row
                .try_get("name")
                .map_err(|_| "could not read server column name".to_string())?,
            logical_type: logical_type(backend, &type_name, &native_type),
            nullable: nullable.eq_ignore_ascii_case("YES"),
            ordinal: u32::try_from(ordinal - 1)
                .map_err(|_| "server column ordinal is out of range".to_string())?,
            default,
            native_type: Some(native_type),
        });
    }

    let primary_rows = query_for_table(key_sql, backend, name)
        .fetch_all(pool)
        .await
        .map_err(|_| "could not inspect server primary key".to_string())?;
    let primary_key = primary_rows
        .iter()
        .map(|row| {
            row.try_get("name")
                .map_err(|_| "could not read server primary key".to_string())
        })
        .collect::<Result<Vec<String>, String>>()?;

    let unique_rows = query_for_table(unique_sql, backend, name)
        .fetch_all(pool)
        .await
        .map_err(|error| {
            let cause = error
                .as_database_error()
                .map(|database_error| database_error.message().to_string())
                .unwrap_or_else(|| error.to_string());
            format!("could not inspect server unique constraints: {cause}")
        })?;
    let unique_groups = grouped_columns(&unique_rows, "name")?;
    let unique = unique_groups.into_values().collect::<Vec<_>>();

    let foreign_rows = query_for_table(foreign_sql, backend, name)
        .fetch_all(pool)
        .await
        .map_err(|_| "could not inspect server foreign keys".to_string())?;
    let foreign_keys = group_foreign_keys(&foreign_rows)?;

    let index_rows = query_for_table(index_sql, backend, name)
        .fetch_all(pool)
        .await
        .map_err(|_| "could not inspect server indexes".to_string())?;
    let indexes = group_indexes(&index_rows)?;

    let trigger_rows = query_for_table(trigger_sql, backend, name)
        .fetch_all(pool)
        .await
        .map_err(|_| "could not inspect server triggers".to_string())?;
    let triggers = trigger_rows
        .iter()
        .map(|row| {
            Ok(DbTriggerSchema {
                name: row
                    .try_get("name")
                    .map_err(|_| "could not read server trigger name".to_string())?,
                sql: row
                    .try_get("definition")
                    .map_err(|_| "could not read server trigger definition".to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(DbTableSchema {
        name: name.to_string(),
        engine,
        columns,
        primary_key,
        unique,
        foreign_keys,
        indexes,
        triggers,
    })
}

fn query_for_table(
    sql: &'static str,
    backend: ServerMigrationBackend,
    table: &str,
) -> sqlx::query::Query<'static, Any, sqlx::any::AnyArguments> {
    let query = sqlx::query::<Any>(sql).bind(table.to_string());
    if backend == ServerMigrationBackend::Mysql && sql.matches('?').count() == 2 {
        query.bind(table.to_string())
    } else {
        query
    }
}

fn grouped_columns(
    rows: &[sqlx::any::AnyRow],
    key_name: &str,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut groups = BTreeMap::<String, Vec<(i64, String)>>::new();
    for row in rows {
        let key: String = row
            .try_get(key_name)
            .map_err(|_| "could not read server constraint name".to_string())?;
        let ordinal: i64 = row
            .try_get("ordinal")
            .map_err(|_| "could not read server constraint column order".to_string())?;
        let column: String = row
            .try_get("column_name")
            .map_err(|_| "could not read server constraint column".to_string())?;
        groups.entry(key).or_default().push((ordinal, column));
    }
    Ok(groups
        .into_iter()
        .map(|(key, mut values)| {
            values.sort_by_key(|(ordinal, _)| *ordinal);
            (key, values.into_iter().map(|(_, column)| column).collect())
        })
        .collect())
}

fn group_foreign_keys(rows: &[sqlx::any::AnyRow]) -> Result<Vec<DbForeignKeySchema>, String> {
    let mut groups = BTreeMap::<String, Vec<(i64, DbForeignKeySchema)>>::new();
    for row in rows {
        let key: String = row
            .try_get("name")
            .map_err(|_| "could not read server foreign key name".to_string())?;
        let ordinal: i64 = row
            .try_get("ordinal")
            .map_err(|_| "could not read server foreign key order".to_string())?;
        let schema = DbForeignKeySchema {
            columns: vec![
                row.try_get("column_name")
                    .map_err(|_| "could not read server foreign key column".to_string())?,
            ],
            referenced_table: row
                .try_get("referenced_table")
                .map_err(|_| "could not read server foreign key target".to_string())?,
            referenced_columns: vec![
                row.try_get("referenced_column")
                    .map_err(|_| "could not read server foreign key target column".to_string())?,
            ],
            on_update: row
                .try_get("on_update")
                .map_err(|_| "could not read server foreign key action".to_string())?,
            on_delete: row
                .try_get("on_delete")
                .map_err(|_| "could not read server foreign key action".to_string())?,
        };
        groups.entry(key).or_default().push((ordinal, schema));
    }
    Ok(groups
        .into_values()
        .map(|mut group| {
            group.sort_by_key(|(ordinal, _)| *ordinal);
            let mut foreign_key = group[0].1.clone();
            foreign_key.columns = group
                .iter()
                .flat_map(|(_, key)| key.columns.clone())
                .collect();
            foreign_key.referenced_columns = group
                .iter()
                .flat_map(|(_, key)| key.referenced_columns.clone())
                .collect();
            foreign_key
        })
        .collect())
}

fn group_indexes(rows: &[sqlx::any::AnyRow]) -> Result<Vec<DbIndexSchema>, String> {
    let mut groups = BTreeMap::<String, Vec<sqlx::any::AnyRow>>::new();
    for row in rows {
        let name: String = row
            .try_get("name")
            .map_err(|_| "could not read server index name".to_string())?;
        groups.entry(name).or_default().push(row.clone());
    }
    groups
        .into_iter()
        .map(|(name, rows)| {
            let first = &rows[0];
            let is_unique: bool = first
                .try_get("is_unique")
                .map_err(|_| "could not read server index uniqueness".to_string())?;
            let partial: bool = first
                .try_get("is_partial")
                .map_err(|_| "could not read server index predicate".to_string())?;
            let mut expression = false;
            let mut ordered = Vec::new();
            for row in rows {
                let ordinal: i64 = row
                    .try_get("ordinal")
                    .map_err(|_| "could not read server index order".to_string())?;
                let column: String = row
                    .try_get("column_name")
                    .map_err(|_| "could not read server index column".to_string())?;
                let has_expression: bool = row
                    .try_get("has_expression")
                    .map_err(|_| "could not read server index expression".to_string())?;
                expression |= has_expression || column.is_empty();
                if !column.is_empty() {
                    ordered.push((ordinal, column));
                }
            }
            ordered.sort_by_key(|(ordinal, _)| *ordinal);
            Ok(DbIndexSchema {
                name,
                columns: ordered.into_iter().map(|(_, column)| column).collect(),
                unique: is_unique,
                partial,
                expression,
            })
        })
        .collect()
}

fn logical_type(backend: ServerMigrationBackend, data_type: &str, native_type: &str) -> String {
    let data_type = data_type.to_ascii_lowercase();
    let native_type = native_type.to_ascii_lowercase();
    match backend {
        ServerMigrationBackend::Postgresql => match data_type.as_str() {
            "boolean" => "boolean",
            "smallint" | "integer" | "bigint" => "i64",
            "real" | "double precision" => "f64",
            "timestamp with time zone" => "instant",
            "bytea" => "bytes",
            _ => "string",
        },
        ServerMigrationBackend::Mysql => {
            if data_type == "tinyint" && native_type.ends_with("(1)") {
                "boolean"
            } else if [
                "tinyint",
                "smallint",
                "mediumint",
                "int",
                "integer",
                "bigint",
            ]
            .iter()
            .any(|kind| data_type == *kind)
            {
                if native_type.contains("unsigned") {
                    "u64"
                } else {
                    "i64"
                }
            } else if ["float", "double", "real", "decimal", "numeric"]
                .iter()
                .any(|kind| data_type == *kind)
            {
                if data_type == "decimal" || data_type == "numeric" {
                    "string"
                } else {
                    "f64"
                }
            } else if data_type == "datetime" {
                "instant"
            } else if [
                "binary",
                "varbinary",
                "blob",
                "tinyblob",
                "mediumblob",
                "longblob",
            ]
            .iter()
            .any(|kind| data_type == *kind)
            {
                "bytes"
            } else {
                "string"
            }
        }
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_list_query_only_selects_engine_for_mysql() {
        let postgres = table_list_query(ServerMigrationBackend::Postgresql);
        let mysql = table_list_query(ServerMigrationBackend::Mysql);

        assert!(postgres.contains("table_name::text AS name"));
        assert!(!postgres.contains("engine"));
        assert!(mysql.contains("table_name AS name, COALESCE(engine, '') AS engine"));
    }

    #[test]
    fn postgres_schema_queries_cast_catalog_names_to_text() {
        let (columns, primary, unique, foreign, indexes, triggers) =
            inspection_queries(ServerMigrationBackend::Postgresql);

        assert!(columns.contains("column_name::text AS name"));
        assert!(columns.contains("udt_name::text AS native_type"));
        assert!(primary.contains("column_name::text AS name"));
        assert!(unique.contains("constraint_name::text AS name"));
        assert!(foreign.contains("conname::text AS name"));
        assert!(indexes.contains("relname::text AS name"));
        assert!(triggers.contains("tgname::text AS name"));
    }

    #[test]
    fn uuid_identifier_columns_keep_string_logical_types_and_native_types_in_snapshots() {
        let postgres_native = "uuid";
        let mysql_uuid_native = "char(36)";
        let mysql_short_native = "bigint unsigned";
        let postgres = DbSchemaSnapshot::new_for_backend(
            DatabaseBackend::Postgresql,
            "./database".to_string(),
            1,
            vec![DbTableSchema {
                name: "users".to_string(),
                engine: None,
                columns: vec![DbColumnSchema {
                    name: "id".to_string(),
                    logical_type: logical_type(
                        ServerMigrationBackend::Postgresql,
                        "uuid",
                        postgres_native,
                    ),
                    nullable: false,
                    ordinal: 0,
                    default: Some("gen_random_uuid()".to_string()),
                    native_type: Some(postgres_native.to_string()),
                }],
                primary_key: vec!["id".to_string()],
                unique: Vec::new(),
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                triggers: Vec::new(),
            }],
        );
        assert!(postgres.validate().is_ok());
        assert_eq!(postgres.tables[0].columns[0].logical_type, "string");
        assert_eq!(
            postgres.tables[0].columns[0].native_type.as_deref(),
            Some("uuid")
        );

        for (native_type, data_type, expected) in [
            (mysql_uuid_native, "char", "string"),
            (mysql_short_native, "bigint", "u64"),
        ] {
            assert_eq!(
                logical_type(ServerMigrationBackend::Mysql, data_type, native_type),
                expected
            );
        }
    }
}
