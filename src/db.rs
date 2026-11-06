use std::collections::BTreeMap;
use std::path::Path;
use std::process;
use std::time::Duration;

use sha2::{Digest, Sha256};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, Row, SqliteConnection};
use tyrus_core::db::{
    DatabaseBackend, DbColumnSchema, DbForeignKeySchema, DbIndexSchema, DbSchemaCache,
    DbSchemaSnapshot, DbTableSchema, DbTriggerSchema,
};
use tyrus_core::project;

#[path = "db/explain_sql.rs"]
mod explain_sql;
#[path = "db/migrate.rs"]
mod migrate;
#[path = "db/server_model_migrations.rs"]
mod server_model_migrations;
#[path = "db/server_schema.rs"]
mod server_schema;

pub fn run(project_root: &Path, args: &[String]) {
    let Some(command) = args.first().map(String::as_str) else {
        usage();
    };
    let result = match command {
        "introspect" if args.len() == 1 => introspect(project_root),
        "check" if args.len() == 1 => check(project_root),
        "diff" if args.len() == 1 => diff(project_root),
        "explain-sql" if args.len() == 1 => explain_sql::run(project_root, None),
        "explain-sql" if args.len() == 2 => explain_sql::run(project_root, Some(&args[1])),
        "migrate" => migrate::run(project_root, &args[1..]),
        "seed" => migrate::run_seed(project_root, &args[1..]),
        "apply" if args.len() == 2 => migrate::apply_seed_file(project_root, &args[1]),
        _ => Err(
            "usage: tyrus db <introspect|check|diff|explain-sql [SQL]|migrate|seed status [name]|apply <name.tyr>".to_string(),
        ),
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn usage() -> ! {
    eprintln!(
        "usage: tyrus db <introspect|check|diff|explain-sql [SQL]|migrate|seed status [name]|apply <name.tyr>"
    );
    process::exit(1);
}

fn introspect(project_root: &Path) -> Result<(), String> {
    let config = load_database_config(project_root)?;
    let snapshot_path = config.resolve_schema_path(project_root)?;
    let snapshot = inspect_configured(project_root, &config)?;
    snapshot.write_atomic(&snapshot_path)?;
    println!(
        "Updated database schema snapshot: {}",
        snapshot_path.display()
    );
    Ok(())
}

fn check(project_root: &Path) -> Result<(), String> {
    let config = load_database_config(project_root)?;
    let snapshot_path = config.resolve_schema_path(project_root)?;
    let mut cache = DbSchemaCache::default();
    let expected = cache
        .load_configured(project_root, &config)
        .map_err(|error| {
            if !snapshot_path.exists() {
                "database schema snapshot is missing; run `tyrus db introspect` first".to_string()
            } else {
                error
            }
        })?;
    let actual = inspect_configured(project_root, &config)?;
    if *expected != actual {
        return Err("database schema drift detected; run `tyrus db introspect` to review and update the snapshot".to_string());
    }
    println!("Database schema matches the snapshot.");
    Ok(())
}

fn diff(project_root: &Path) -> Result<(), String> {
    let config = load_database_config(project_root)?;
    let snapshot_path = config.resolve_schema_path(project_root)?;
    let mut cache = DbSchemaCache::default();
    let expected = cache
        .load_configured(project_root, &config)
        .map_err(|error| {
            if !snapshot_path.exists() {
                "database schema snapshot is missing; run `tyrus db introspect` first".to_string()
            } else {
                error
            }
        })?;
    let actual = inspect_configured(project_root, &config)?;
    let changes = schema_diff(&expected, &actual);
    if changes.is_empty() {
        println!("Database schema matches the snapshot.");
    } else {
        for change in changes {
            println!("{change}");
        }
        println!("Review the changes, then run `tyrus db introspect` to update the snapshot.");
    }
    match config.backend {
        DatabaseBackend::Sqlite => migrate::diff_models(project_root, &expected)?,
        DatabaseBackend::Postgresql | DatabaseBackend::Mysql => {
            server_model_migrations::diff_models(project_root, &expected)?
        }
        DatabaseBackend::Mongodb => {}
    }
    Ok(())
}

fn schema_diff(expected: &DbSchemaSnapshot, actual: &DbSchemaSnapshot) -> Vec<String> {
    let expected_tables = expected
        .tables
        .iter()
        .map(|table| (table.name.as_str(), table))
        .collect::<BTreeMap<_, _>>();
    let actual_tables = actual
        .tables
        .iter()
        .map(|table| (table.name.as_str(), table))
        .collect::<BTreeMap<_, _>>();
    let mut changes = Vec::new();
    if expected.backend != actual.backend {
        changes.push(format!(
            "~ backend: {:?} -> {:?}",
            expected.backend, actual.backend
        ));
    }
    if expected.source != actual.source {
        changes.push("~ configured database target changed".to_string());
    }
    if expected.schema_version != actual.schema_version {
        changes.push(format!(
            "~ database schema version: {} -> {}",
            expected.schema_version, actual.schema_version
        ));
    }
    for (name, table) in &expected_tables {
        if !actual_tables.contains_key(name) {
            changes.push(format!("- table `{}`", table.name));
        }
    }
    for (name, table) in &actual_tables {
        if !expected_tables.contains_key(name) {
            changes.push(format!("+ table `{}`", table.name));
        }
    }
    for (name, expected_table) in &expected_tables {
        let Some(actual_table) = actual_tables.get(name) else {
            continue;
        };
        diff_table(expected_table, actual_table, &mut changes);
    }
    changes
}

fn diff_table(expected: &DbTableSchema, actual: &DbTableSchema, changes: &mut Vec<String>) {
    let expected_columns = expected
        .columns
        .iter()
        .map(|column| (column.name.as_str(), column))
        .collect::<BTreeMap<_, _>>();
    let actual_columns = actual
        .columns
        .iter()
        .map(|column| (column.name.as_str(), column))
        .collect::<BTreeMap<_, _>>();
    for (name, column) in &expected_columns {
        if !actual_columns.contains_key(name) {
            changes.push(format!("- column `{}.{}`", expected.name, column.name));
        }
    }
    for (name, column) in &actual_columns {
        if !expected_columns.contains_key(name) {
            changes.push(format!(
                "+ column `{}.{}` ({})",
                actual.name, column.name, column.logical_type
            ));
        }
    }
    for (name, expected_column) in &expected_columns {
        let Some(actual_column) = actual_columns.get(name) else {
            continue;
        };
        if expected_column.logical_type != actual_column.logical_type {
            changes.push(format!(
                "~ column `{}.{}` type: {} -> {}",
                expected.name,
                expected_column.name,
                expected_column.logical_type,
                actual_column.logical_type
            ));
        }
        if expected_column.native_type != actual_column.native_type {
            changes.push(format!(
                "~ column `{}.{}` native type: {:?} -> {:?}",
                expected.name,
                expected_column.name,
                expected_column.native_type,
                actual_column.native_type
            ));
        }
        if expected_column.nullable != actual_column.nullable {
            changes.push(format!(
                "~ column `{}.{}` nullable: {} -> {}",
                expected.name,
                expected_column.name,
                expected_column.nullable,
                actual_column.nullable
            ));
        }
        if expected_column.default != actual_column.default {
            changes.push(format!(
                "~ column `{}.{}` default expression changed",
                expected.name, expected_column.name
            ));
        }
    }
    if expected.primary_key != actual.primary_key {
        changes.push(format!(
            "~ table `{}` primary key: {:?} -> {:?}",
            expected.name, expected.primary_key, actual.primary_key
        ));
    }
    if expected.engine != actual.engine {
        changes.push(format!(
            "~ table `{}` engine: {:?} -> {:?}",
            expected.name, expected.engine, actual.engine
        ));
    }
    if expected.unique != actual.unique {
        changes.push(format!(
            "~ table `{}` unique constraints changed",
            expected.name
        ));
    }
    if expected.foreign_keys != actual.foreign_keys {
        changes.push(format!("~ table `{}` foreign keys changed", expected.name));
    }
    if expected.indexes != actual.indexes {
        changes.push(format!("~ table `{}` indexes changed", expected.name));
    }
    let expected_triggers = expected
        .triggers
        .iter()
        .map(|trigger| (trigger.name.as_str(), trigger))
        .collect::<BTreeMap<_, _>>();
    let actual_triggers = actual
        .triggers
        .iter()
        .map(|trigger| (trigger.name.as_str(), trigger))
        .collect::<BTreeMap<_, _>>();
    for (name, trigger) in &expected_triggers {
        if !actual_triggers.contains_key(name) {
            changes.push(format!("- trigger `{}.{}`", expected.name, trigger.name));
        }
    }
    for (name, trigger) in &actual_triggers {
        if !expected_triggers.contains_key(name) {
            changes.push(format!("+ trigger `{}.{}`", actual.name, trigger.name));
        }
    }
    for (name, expected_trigger) in &expected_triggers {
        if let Some(actual_trigger) = actual_triggers.get(name)
            && expected_trigger.sql != actual_trigger.sql
        {
            changes.push(format!(
                "~ trigger `{}.{}` definition changed",
                expected.name, expected_trigger.name
            ));
        }
    }
}

fn resolve_config(
    project_root: &Path,
) -> Result<
    (
        tyrus_core::db::DatabaseConfig,
        std::path::PathBuf,
        std::path::PathBuf,
    ),
    String,
> {
    let config = load_database_config(project_root)?;
    if config.backend != DatabaseBackend::Sqlite {
        return Err("this database operation currently requires SQLite".to_string());
    }
    let database = config.resolve_database_path(project_root)?;
    let snapshot = config.resolve_schema_path(project_root)?;
    Ok((config, database, snapshot))
}

pub(super) fn inspect_configured(
    project_root: &Path,
    config: &tyrus_core::db::DatabaseConfig,
) -> Result<DbSchemaSnapshot, String> {
    match config.backend {
        DatabaseBackend::Sqlite => {
            let database_path = config.resolve_database_path(project_root)?;
            inspect_sqlite(&database_path, config.path.clone())
        }
        DatabaseBackend::Postgresql | DatabaseBackend::Mysql => {
            inspect_server(project_root, config)
        }
        DatabaseBackend::Mongodb => {
            Err("schema introspection is not available for MongoDB".to_string())
        }
    }
}

fn inspect_server(
    project_root: &Path,
    config: &tyrus_core::db::DatabaseConfig,
) -> Result<DbSchemaSnapshot, String> {
    let backend = match config.backend {
        DatabaseBackend::Postgresql => migrate::ServerMigrationBackend::Postgresql,
        DatabaseBackend::Mysql => migrate::ServerMigrationBackend::Mysql,
        _ => return Err("server schema introspection requires PostgreSQL or MySQL".to_string()),
    };
    let connection_string =
        zeroize::Zeroizing::new(migrate::server_connection_string(project_root, config)?);
    let runtime_target: serde_json::Value =
        serde_json::from_str(&config.runtime_target(project_root)?)
            .map_err(|_| "could not resolve server database settings".to_string())?;
    let ca_file = runtime_target
        .get("tls")
        .and_then(|tls| tls.get("caFile"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let cert_file = runtime_target
        .get("tls")
        .and_then(|tls| tls.get("certFile"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let key_file = runtime_target
        .get("tls")
        .and_then(|tls| tls.get("keyFile"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let source = server_snapshot_source(backend, &connection_string)?;
    let query_timeout_ms = config
        .pool
        .as_ref()
        .and_then(|pool| pool.query_timeout_ms)
        .unwrap_or(30_000);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not initialize server schema inspection".to_string())?;
    runtime.block_on(async move {
        let pool = migrate::open_server_migration_pool(
            backend,
            &connection_string,
            config.pool.as_ref(),
            ca_file.as_deref(),
            cert_file.as_deref(),
            key_file.as_deref(),
        )
        .await?;
        tokio::time::timeout(
            Duration::from_millis(query_timeout_ms),
            server_schema::inspect(&pool, backend, source),
        )
        .await
        .map_err(|_| "server schema inspection timed out".to_string())?
    })
}

fn server_snapshot_source(
    backend: migrate::ServerMigrationBackend,
    connection_string: &str,
) -> Result<String, String> {
    let options = connection_string
        .parse::<sqlx::any::AnyConnectOptions>()
        .map_err(|_| "database connection variable is not a valid server URL".to_string())?;
    let url = options.database_url;
    let host = url
        .host_str()
        .ok_or_else(|| "database connection variable has no server host".to_string())?;
    let database = url.path();
    let backend_name = match backend {
        migrate::ServerMigrationBackend::Postgresql => "postgresql",
        migrate::ServerMigrationBackend::Mysql => "mysql",
    };
    let mut digest = Sha256::new();
    digest.update(backend_name.as_bytes());
    digest.update([0]);
    digest.update(host.as_bytes());
    digest.update([0]);
    digest.update(database.as_bytes());
    digest.update(url.port().unwrap_or_default().to_be_bytes());
    let hash = digest.finalize();
    let mut source = format!("{backend_name}-");
    for byte in hash {
        use std::fmt::Write as _;
        let _ = write!(source, "{byte:02x}");
    }
    Ok(source)
}

fn load_database_config(project_root: &Path) -> Result<tyrus_core::db::DatabaseConfig, String> {
    project::load_project(project_root)
        .map_err(|_| "could not load tyrus.json database configuration".to_string())?
        .database
        .ok_or_else(|| "tyrus.json has no `database` configuration".to_string())
}

fn inspect_sqlite(database_path: &Path, source: String) -> Result<DbSchemaSnapshot, String> {
    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .read_only(true)
        .create_if_missing(false);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not initialize SQLite inspection".to_string())?;
    runtime.block_on(async move {
        let mut connection = SqliteConnection::connect_with(&options)
            .await
            .map_err(|_| "could not open SQLite database in read-only mode".to_string())?;
        inspect_connection(&mut connection, source).await
    })
}

async fn inspect_connection(
    connection: &mut SqliteConnection,
    source: String,
) -> Result<DbSchemaSnapshot, String> {
    let version: i64 = sqlx::query_scalar("PRAGMA schema_version")
        .fetch_one(&mut *connection)
        .await
        .map_err(|_| "could not read SQLite schema version".to_string())?;
    let table_rows = sqlx::query("SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .fetch_all(&mut *connection)
        .await
        .map_err(|_| "could not list SQLite tables".to_string())?;
    let mut tables = Vec::with_capacity(table_rows.len());
    for row in table_rows {
        let name: String = row
            .try_get("name")
            .map_err(|_| "could not read SQLite table name".to_string())?;
        tables.push(inspect_table(connection, name).await?);
    }
    tables.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(DbSchemaSnapshot::new(source, version, tables))
}

async fn inspect_table(
    connection: &mut SqliteConnection,
    name: String,
) -> Result<DbTableSchema, String> {
    let quoted = quote_identifier(&name);
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("PRAGMA table_xinfo({quoted})")))
        .fetch_all(&mut *connection)
        .await
        .map_err(|_| "could not inspect SQLite table columns".to_string())?;
    let mut columns = Vec::new();
    let mut primary = BTreeMap::new();
    for row in rows {
        let hidden: i64 = row
            .try_get("hidden")
            .map_err(|_| "could not read SQLite column metadata".to_string())?;
        if hidden == 1 {
            continue;
        }
        let ordinal: i64 = row
            .try_get("cid")
            .map_err(|_| "could not read SQLite column ordinal".to_string())?;
        let column_name: String = row
            .try_get("name")
            .map_err(|_| "could not read SQLite column name".to_string())?;
        let declared_type: String = row
            .try_get("type")
            .map_err(|_| "could not read SQLite column type".to_string())?;
        let not_null: i64 = row
            .try_get("notnull")
            .map_err(|_| "could not read SQLite nullability".to_string())?;
        let pk: i64 = row
            .try_get("pk")
            .map_err(|_| "could not read SQLite primary key".to_string())?;
        let default: Option<String> = row
            .try_get("dflt_value")
            .map_err(|_| "could not read SQLite column default".to_string())?;
        if pk > 0 {
            primary.insert(pk, column_name.clone());
        }
        columns.push(DbColumnSchema {
            name: column_name,
            logical_type: sqlite_logical_type(&declared_type),
            nullable: not_null == 0 && pk == 0,
            ordinal: u32::try_from(ordinal)
                .map_err(|_| "SQLite column ordinal is out of range".to_string())?,
            default,
            native_type: None,
        });
    }
    columns.sort_by_key(|column| column.ordinal);
    let primary_key = primary.into_values().collect::<Vec<_>>();
    let mut foreign_groups: BTreeMap<i64, Vec<(i64, DbForeignKeySchema)>> = BTreeMap::new();
    let mut referenced_primary_keys: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in sqlx::query(sqlx::AssertSqlSafe(format!(
        "PRAGMA foreign_key_list({quoted})"
    )))
    .fetch_all(&mut *connection)
    .await
    .map_err(|_| "could not inspect SQLite foreign keys".to_string())?
    {
        let id: i64 = row
            .try_get("id")
            .map_err(|_| "could not read SQLite foreign key".to_string())?;
        let seq: i64 = row
            .try_get("seq")
            .map_err(|_| "could not read SQLite foreign key order".to_string())?;
        let referenced_table: String = row
            .try_get("table")
            .map_err(|_| "could not read SQLite referenced table".to_string())?;
        let from: String = row
            .try_get("from")
            .map_err(|_| "could not read SQLite foreign key column".to_string())?;
        let to: Option<String> = row
            .try_get("to")
            .map_err(|_| "could not read SQLite referenced column".to_string())?;
        let referenced_column = if let Some(column) = to {
            column
        } else {
            if !referenced_primary_keys.contains_key(&referenced_table) {
                let target_quoted = quote_identifier(&referenced_table);
                let target_rows = sqlx::query(sqlx::AssertSqlSafe(format!(
                    "PRAGMA table_xinfo({target_quoted})"
                )))
                .fetch_all(&mut *connection)
                .await
                .map_err(|_| "could not inspect SQLite referenced primary key".to_string())?;
                let mut target_key = BTreeMap::new();
                for target_row in target_rows {
                    let ordinal: i64 = target_row
                        .try_get("pk")
                        .map_err(|_| "could not read SQLite referenced primary key".to_string())?;
                    if ordinal > 0 {
                        let column: String = target_row.try_get("name").map_err(|_| {
                            "could not read SQLite referenced primary key column".to_string()
                        })?;
                        target_key.insert(ordinal, column);
                    }
                }
                referenced_primary_keys
                    .insert(referenced_table.clone(), target_key.into_values().collect());
            }
            let seq_index = usize::try_from(seq)
                .map_err(|_| "SQLite foreign key order is out of range".to_string())?;
            referenced_primary_keys
                .get(&referenced_table)
                .and_then(|columns| columns.get(seq_index))
                .cloned()
                .ok_or_else(|| {
                    "SQLite foreign key does not match the referenced primary key".to_string()
                })?
        };
        let on_update: String = row
            .try_get("on_update")
            .map_err(|_| "could not read SQLite foreign key action".to_string())?;
        let on_delete: String = row
            .try_get("on_delete")
            .map_err(|_| "could not read SQLite foreign key action".to_string())?;
        foreign_groups.entry(id).or_default().push((
            seq,
            DbForeignKeySchema {
                columns: vec![from],
                referenced_table,
                referenced_columns: vec![referenced_column],
                on_update,
                on_delete,
            },
        ));
    }
    let foreign_keys = foreign_groups
        .into_values()
        .map(|mut group| {
            group.sort_by_key(|(seq, _)| *seq);
            let mut fk = group[0].1.clone();
            fk.columns = group.iter().map(|(_, fk)| fk.columns[0].clone()).collect();
            fk.referenced_columns = group
                .iter()
                .map(|(_, fk)| fk.referenced_columns[0].clone())
                .collect();
            fk
        })
        .collect();
    let mut indexes = Vec::new();
    let mut unique = Vec::new();
    for row in sqlx::query(sqlx::AssertSqlSafe(format!("PRAGMA index_list({quoted})")))
        .fetch_all(&mut *connection)
        .await
        .map_err(|_| "could not inspect SQLite indexes".to_string())?
    {
        let index_name: String = row
            .try_get("name")
            .map_err(|_| "could not read SQLite index name".to_string())?;
        let is_unique: i64 = row
            .try_get("unique")
            .map_err(|_| "could not read SQLite index uniqueness".to_string())?;
        let origin: String = row
            .try_get("origin")
            .map_err(|_| "could not read SQLite index origin".to_string())?;
        if origin == "pk" {
            continue;
        }
        let partial: i64 = row
            .try_get("partial")
            .map_err(|_| "could not read SQLite index metadata".to_string())?;
        let index_quoted = quote_identifier(&index_name);
        let mut index_columns = Vec::new();
        let mut expression = false;
        for item in sqlx::query(sqlx::AssertSqlSafe(format!(
            "PRAGMA index_info({index_quoted})"
        )))
        .fetch_all(&mut *connection)
        .await
        .map_err(|_| "could not inspect SQLite index columns".to_string())?
        {
            let column: Option<String> = item
                .try_get("name")
                .map_err(|_| "could not read SQLite index column".to_string())?;
            if let Some(column) = column {
                index_columns.push(column);
            } else {
                expression = true;
            }
        }
        if is_unique != 0 && !index_columns.is_empty() {
            unique.push(index_columns.clone());
        }
        indexes.push(DbIndexSchema {
            name: index_name,
            columns: index_columns,
            unique: is_unique != 0,
            partial: partial != 0,
            expression,
        });
    }
    unique.sort();
    indexes.sort_by(|left, right| left.name.cmp(&right.name));
    let mut triggers = Vec::new();
    for row in sqlx::query(
        "SELECT name, sql FROM sqlite_schema WHERE type = 'trigger' AND tbl_name = ? AND sql IS NOT NULL ORDER BY name",
    )
    .bind(&name)
    .fetch_all(&mut *connection)
    .await
    .map_err(|_| "could not inspect SQLite table triggers".to_string())?
    {
        triggers.push(DbTriggerSchema {
            name: row
                .try_get("name")
                .map_err(|_| "could not read SQLite trigger name".to_string())?,
            sql: row
                .try_get("sql")
                .map_err(|_| "could not read SQLite trigger definition".to_string())?,
        });
    }
    Ok(DbTableSchema {
        name,
        engine: None,
        columns,
        primary_key,
        unique,
        foreign_keys,
        indexes,
        triggers,
    })
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn sqlite_logical_type(declared: &str) -> String {
    let upper = declared.to_ascii_uppercase();
    if upper.contains("INT") {
        "i64".to_string()
    } else if ["REAL", "FLOA", "DOUB"]
        .iter()
        .any(|part| upper.contains(part))
    {
        "f64".to_string()
    } else if upper.contains("BLOB") || upper.is_empty() {
        "bytes".to_string()
    } else {
        "string".to_string()
    }
}
