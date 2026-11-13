use super::{load_database_config, resolve_config};
use sha2::{Digest, Sha256};
use sqlx::any::{AnyConnectOptions, AnyPoolOptions};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Any, Connection, Pool, Row, SqliteConnection};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[path = "migrate/model_schema.rs"]
mod model_schema;
#[path = "migrate/mongodb_seeds.rs"]
mod mongodb_seeds;
#[path = "migrate/tls.rs"]
mod tls;
pub(super) use model_schema::{
    diff_models, generate_model_schema, generate_server_model_schema, quote_identifier_for_backend,
    server_default, server_sql_type, validate_initial_model_foreign_keys,
};
use model_schema::{
    generate_incremental_schema, inspect_sqlite_rebuild_metadata,
    validate_generated_schema_migration,
};

const MAX_MIGRATION_BYTES: u64 = 4 * 1024 * 1024;
const HISTORY_TABLE: &str = "_tyrus_migrations";
const SEED_HISTORY_TABLE: &str = "_tyrus_seeds";
static NEXT_ARTIFACT_TEMP: AtomicU64 = AtomicU64::new(0);

struct Migration {
    version: i64,
    name: String,
    sql: String,
    checksum: String,
}

struct Seed {
    name: String,
    checksum: String,
}

pub(super) fn run(project_root: &Path, args: &[String]) -> Result<(), String> {
    match args {
        [command, name] if command == "create" => create(project_root, name),
        [command, name] if command == "generate" => generate_from_models(project_root, name),
        [command] if command == "status" => run_async(project_root, false),
        [command] if command == "apply" => run_async(project_root, true),
        _ => {
            Err("usage: tyrus db migrate <create <name>|generate <name>|status|apply>".to_string())
        }
    }
}

fn generate_from_models(project_root: &Path, name: &str) -> Result<(), String> {
    let database_config = load_database_config(project_root)?;
    if matches!(
        database_config.backend,
        tyrus_core::db::DatabaseBackend::Postgresql | tyrus_core::db::DatabaseBackend::Mysql
    ) {
        validate_name(name)?;
        let dir = migration_dir(project_root)?;
        let migrations = load_migrations(&dir)?;
        let entry = project_root.join("src/main.tyr");
        let program = tyrus_core::module_loader::load_program_for_db_migrations(&entry)
            .map_err(|error| format!("could not load project models: {error}"))?;
        validate_initial_model_foreign_keys(&program.items, false)?;
        if !migrations.is_empty() {
            return super::server_model_migrations::generate(
                project_root,
                name,
                &database_config,
                &program.items,
            );
        }
        let sql = generate_server_model_schema(&program.items, &database_config.backend)?;
        if sql.is_empty() {
            return Err("no `@model` declarations were found in the project".to_string());
        }
        return create_with_sql(project_root, name, &sql);
    }
    if database_config.backend == tyrus_core::db::DatabaseBackend::Mongodb {
        return Err("model migration generation is not available for MongoDB".to_string());
    }
    let (config, database_path, snapshot_path) = resolve_config(project_root)?;
    validate_name(name).map_err(|_| "migration name must start with a lowercase letter and contain only lowercase letters, digits, or underscores".to_string())?;
    let entry = project_root.join("src/main.tyr");
    let program = tyrus_core::module_loader::load_program_for_db_migrations(&entry)
        .map_err(|error| format!("could not load project models: {error}"))?;
    let sql = if database_path.exists() {
        let snapshot = tyrus_core::db::DbSchemaSnapshot::load(&snapshot_path).map_err(|_| {
            "database schema snapshot is missing or invalid; run `tyrus db introspect` first"
                .to_string()
        })?;
        let actual = super::inspect_sqlite(&database_path, config.path)?;
        if snapshot.schema_version != actual.schema_version || snapshot.tables != actual.tables {
            return Err("database schema drift detected; run `tyrus db diff` and `tyrus db introspect` before generating a model migration".to_string());
        }
        let metadata = inspect_sqlite_rebuild_metadata(&database_path)?;
        let migration = generate_incremental_schema(
            &program.items,
            &snapshot.tables,
            &metadata.views,
            &metadata.indexes_by_table,
            &metadata.triggers_by_table,
        )?;
        validate_generated_schema_migration(&metadata.schema_sql, &migration)?;
        migration
    } else {
        if snapshot_path.exists() {
            return Err(
                "database schema snapshot exists but the configured database file is missing"
                    .to_string(),
            );
        }
        validate_initial_model_foreign_keys(&program.items, true)?;
        let sql = generate_model_schema(&program.items)?;
        validate_generated_schema_migration(&[], &sql)?;
        sql
    };
    if sql.is_empty() {
        return Err("no `@model` declarations were found in the project".to_string());
    }
    create_with_sql(project_root, name, &sql)
}

pub(super) fn create_with_sql(project_root: &Path, name: &str, sql: &str) -> Result<(), String> {
    let dir = migration_dir(project_root)?;
    fs::create_dir_all(&dir).map_err(|_| "could not create migrations directory".to_string())?;
    let next = load_migrations(&dir)?.last().map_or(Ok(1), |migration| {
        migration
            .version
            .checked_add(1)
            .ok_or_else(|| "migration version is out of range".to_string())
    })?;
    let path = dir.join(format!("{next:04}_{name}.sql"));
    write_new_file_atomically(&path, sql.as_bytes(), "migration")?;
    println!("Created model migration: {}", path.display());
    Ok(())
}

pub(super) fn run_seed(project_root: &Path, args: &[String]) -> Result<(), String> {
    match args {
        [command] if command == "status" => run_seed_async(project_root, None),
        [command, name] if command == "status" => run_seed_async(project_root, Some(name)),
        _ => Err(
            "usage: tyrus db seed status [name]; use `tyrus db apply <name.tyr>` to run a seed"
                .to_string(),
        ),
    }
}

pub(super) fn apply_seed_file(project_root: &Path, file_name: &str) -> Result<(), String> {
    let name = file_name
        .strip_suffix(".tyr")
        .ok_or_else(|| "seed file must use the `.tyr` extension".to_string())?;
    validate_name(name)
        .map_err(|_| "seed file must be a name such as `default_roles.tyr`".to_string())?;
    let seeds = load_seeds(&seed_dir(project_root)?)?;
    let seed = seeds
        .iter()
        .find(|seed| seed.name == name)
        .ok_or_else(|| format!("seed `{file_name}` was not found in db/seeds"))?;
    let config = load_database_config(project_root)?;
    match config.backend {
        tyrus_core::db::DatabaseBackend::Sqlite => {
            let (_, path, _) = resolve_config(project_root)?;
            if path == Path::new(":memory:") || !path.is_file() {
                return Err("apply database migrations before applying a seed".to_string());
            }
            run_tyr_seed_sqlite(project_root, &path, &seeds, seed)
        }
        tyrus_core::db::DatabaseBackend::Postgresql | tyrus_core::db::DatabaseBackend::Mysql => {
            run_tyr_seed_server(project_root, config, &seeds, seed)
        }
        tyrus_core::db::DatabaseBackend::Mongodb => {
            mongodb_seeds::apply(project_root, &config, &seeds, seed)
        }
    }
}

fn run_seed_program(project_root: &Path, name: &str) -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|_| "could not locate the running Tyrus executable".to_string())?;
    let file = format!("db/seeds/{name}.tyr");
    let status = std::process::Command::new(executable)
        .arg("run")
        .arg(file)
        .current_dir(project_root)
        .status()
        .map_err(|_| "could not start the Tyrus seed program".to_string())?;
    if !status.success() {
        return Err(format!(
            "seed `{name}.tyr` failed with {}; its history remains incomplete, inspect the database before retrying",
            status.code().map_or_else(
                || "a signal".to_string(),
                |code| format!("exit code {code}")
            )
        ));
    }
    Ok(())
}

fn run_tyr_seed_sqlite(
    project_root: &Path,
    path: &Path,
    seeds: &[Seed],
    seed: &Seed,
) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not initialize SQLite seed history".to_string())?;
    runtime.block_on(async {
        let mut connection = SqliteConnection::connect_with(&connect_options(path, true))
            .await.map_err(|_| "could not open SQLite database for seed history".to_string())?;
        let migrations = load_migrations(&migration_dir(project_root)?)?;
        if !migrations.is_empty() {
            let applied = history(&mut connection).await?;
            verify_history(&migrations, &applied)?;
            if applied.len() != migrations.len() {
                return Err("apply all pending migrations before applying seeds".to_string());
            }
        }
        sqlx::query("CREATE TABLE IF NOT EXISTS _tyrus_seeds (name TEXT PRIMARY KEY, checksum TEXT NOT NULL, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)")
            .execute(&mut connection).await.map_err(|_| "could not create SQLite seed history".to_string())?;
        let applied = seed_history(&mut connection).await?;
        verify_seed_history(seeds, &applied)?;
        if let Some(checksum) = applied.get(&seed.name) {
            if checksum == &seed.checksum { return Err(format!("seed `{}` is already applied", seed.name)); }
            if checksum == &format!("started:{}", seed.checksum) {
                return Err(format!("seed `{}` is in an incomplete state; inspect the database before retrying", seed.name));
            }
            return Err(format!("applied seed `{}` has changed; restore its original file and add a new seed", seed.name));
        }
        let started = format!("started:{}", seed.checksum);
        sqlx::query("INSERT INTO _tyrus_seeds (name, checksum) VALUES (?, ?)")
            .bind(&seed.name).bind(&started).execute(&mut connection).await
            .map_err(|_| "could not reserve SQLite seed history; another seed operation may be running".to_string())?;
        connection.close().await.map_err(|_| "could not close SQLite seed history connection".to_string())?;
        run_seed_program(project_root, &seed.name)?;
        let mut connection = SqliteConnection::connect_with(&connect_options(path, true))
            .await.map_err(|_| "could not reopen SQLite database to finalize seed history".to_string())?;
        let finalized = sqlx::query("UPDATE _tyrus_seeds SET checksum = ?, applied_at = CURRENT_TIMESTAMP WHERE name = ? AND checksum = ?")
            .bind(&seed.checksum).bind(&seed.name).bind(&started).execute(&mut connection).await
            .map_err(|_| "could not finalize SQLite seed history".to_string())?;
        if finalized.rows_affected() != 1 {
            return Err("SQLite seed history changed while the seed program was running".to_string());
        }
        connection.close().await.map_err(|_| "could not close SQLite seed history connection".to_string())?;
        println!("applied: {}", seed.name);
        Ok(())
    })
}

fn run_tyr_seed_server(
    project_root: &Path,
    database_config: tyrus_core::db::DatabaseConfig,
    seeds: &[Seed],
    seed: &Seed,
) -> Result<(), String> {
    let backend = match database_config.backend {
        tyrus_core::db::DatabaseBackend::Postgresql => ServerMigrationBackend::Postgresql,
        tyrus_core::db::DatabaseBackend::Mysql => ServerMigrationBackend::Mysql,
        _ => unreachable!(),
    };
    let connection_string =
        zeroize::Zeroizing::new(server_connection_string(project_root, &database_config)?);
    let runtime_target: serde_json::Value =
        serde_json::from_str(&database_config.runtime_target(project_root)?)
            .map_err(|_| "could not resolve server database settings".to_string())?;
    let tls = runtime_target.get("tls");
    let ca_file = tls
        .and_then(|value| value.get("caFile"))
        .and_then(serde_json::Value::as_str);
    let cert_file = tls
        .and_then(|value| value.get("certFile"))
        .and_then(serde_json::Value::as_str);
    let key_file = tls
        .and_then(|value| value.get("keyFile"))
        .and_then(serde_json::Value::as_str);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not initialize server seed history".to_string())?;
    runtime.block_on(async {
        let pool = open_server_migration_pool(
            backend,
            &connection_string,
            database_config.pool.as_ref(),
            ca_file,
            cert_file,
            key_file,
        )
        .await?;
        let migrations = load_migrations(&migration_dir(project_root)?)?;
        let result = async {
            let mut connection = pool.acquire().await
                .map_err(|_| "could not acquire server seed history connection".to_string())?;
            verify_server_migrations_applied(&mut connection, backend, &migrations).await?;
            sqlx::query("CREATE TABLE IF NOT EXISTS _tyrus_seeds (name VARCHAR(255) PRIMARY KEY, checksum CHAR(64) NOT NULL, state VARCHAR(16) NOT NULL DEFAULT 'applied', applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP)")
                .execute(&mut *connection).await
                .map_err(|_| "could not create server seed history".to_string())?;
            let rows = sqlx::query(server_seed_history_select(backend))
                .fetch_all(&mut *connection).await
                .map_err(|_| "could not read server seed history".to_string())?;
            let mut applied = BTreeMap::new();
            for row in rows {
                let name: String = row.try_get("name").map_err(|_| "invalid server seed name".to_string())?;
                let checksum: String = row.try_get("checksum").map_err(|_| "invalid server seed checksum".to_string())?;
                let state: String = row.try_get("state").map_err(|_| "invalid server seed state".to_string())?;
                if state != "applied" {
                    return Err(format!("seed '{}' is in an incomplete state; inspect the database before retrying", name));
                }
                if applied.insert(name, checksum).is_some() {
                    return Err("server seed history contains duplicate names".to_string());
                }
            }
            verify_seed_history(seeds, &applied)?;
            let row = sqlx::query(server_seed_lookup_select(backend))
                .bind(&seed.name).fetch_optional(&mut *connection).await
                .map_err(|_| "could not read server seed history".to_string())?;
            if let Some(row) = row {
                let checksum: String = row.try_get("checksum")
                    .map_err(|_| "invalid server seed checksum".to_string())?;
                let state: String = row.try_get("state")
                    .map_err(|_| "invalid server seed state".to_string())?;
                if state != "applied" {
                    return Err(format!("seed '{}' is in an incomplete state; inspect the database before retrying", seed.name));
                }
                if checksum == seed.checksum {
                    return Err(format!("seed '{}' is already applied", seed.name));
                }
                return Err(format!("applied seed '{}' has changed; restore its original file and add a new seed", seed.name));
            }
            let (reserve_sql, finish_sql) = match backend {
                ServerMigrationBackend::Postgresql => (
                    "INSERT INTO _tyrus_seeds (name, checksum, state) VALUES ($1, $2, 'started')",
                    "UPDATE _tyrus_seeds SET state = 'applied', applied_at = CURRENT_TIMESTAMP WHERE name = $1 AND checksum = $2",
                ),
                ServerMigrationBackend::Mysql => (
                    "INSERT INTO _tyrus_seeds (name, checksum, state) VALUES (?, ?, 'started')",
                    "UPDATE _tyrus_seeds SET state = 'applied', applied_at = CURRENT_TIMESTAMP WHERE name = ? AND checksum = ?",
                ),
            };
            sqlx::query(reserve_sql).bind(&seed.name).bind(&seed.checksum)
                .execute(&mut *connection).await
                .map_err(|_| "could not reserve server seed history; another seed operation may be running".to_string())?;
            drop(connection);
            run_seed_program(project_root, &seed.name)?;
            let mut connection = pool.acquire().await
                .map_err(|_| "could not reacquire server seed history connection".to_string())?;
            let finalized = sqlx::query(finish_sql).bind(&seed.name).bind(&seed.checksum)
                .execute(&mut *connection).await
                .map_err(|_| "could not finalize server seed history".to_string())?;
            if finalized.rows_affected() != 1 {
                return Err("server seed history changed while the seed program was running".to_string());
            }
            println!("applied: {}", seed.name);
            Ok(())
        }.await;
        pool.close().await;
        result
    })
}

fn migration_dir(project_root: &Path) -> Result<PathBuf, String> {
    let root = project_root
        .canonicalize()
        .map_err(|_| "could not resolve the project root".to_string())?;
    let db_dir = root.join("db");
    let dir = db_dir.join("migrations");
    for path in [&db_dir, &dir] {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("could not inspect the migrations directory".to_string()),
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(
                "migrations directory must be a regular directory inside the project".to_string(),
            );
        }
    }
    Ok(dir)
}

fn seed_dir(project_root: &Path) -> Result<PathBuf, String> {
    let root = project_root
        .canonicalize()
        .map_err(|_| "could not resolve the project root".to_string())?;
    let db_dir = root.join("db");
    let dir = db_dir.join("seeds");
    for path in [&db_dir, &dir] {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("could not inspect the seeds directory".to_string()),
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(
                "seeds directory must be a regular directory inside the project".to_string(),
            );
        }
    }
    Ok(dir)
}

fn create(project_root: &Path, name: &str) -> Result<(), String> {
    let config = load_database_config(project_root)?;
    if config.backend == tyrus_core::db::DatabaseBackend::Mongodb {
        return Err(
            "SQL migration files are available only for SQLite, PostgreSQL, and MySQL".to_string(),
        );
    }
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        || !name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
    {
        return Err("migration name must start with a lowercase letter and contain only lowercase letters, digits, or underscores".to_string());
    }
    let dir = migration_dir(project_root)?;
    fs::create_dir_all(&dir).map_err(|_| "could not create migrations directory".to_string())?;
    let next = load_migrations(&dir)?.last().map_or(Ok(1), |migration| {
        migration
            .version
            .checked_add(1)
            .ok_or_else(|| "migration version is out of range".to_string())
    })?;
    let path = dir.join(format!("{next:04}_{name}.sql"));
    let header = match config.backend {
        tyrus_core::db::DatabaseBackend::Sqlite => "-- Write SQLite schema changes here.\n",
        tyrus_core::db::DatabaseBackend::Postgresql => "-- Write PostgreSQL schema changes here.\n",
        tyrus_core::db::DatabaseBackend::Mysql => "-- Write MySQL schema changes here.\n",
        tyrus_core::db::DatabaseBackend::Mongodb => unreachable!(),
    };
    write_new_file_atomically(&path, header.as_bytes(), "migration")?;
    println!("Created migration: {}", path.display());
    Ok(())
}

fn load_migrations(dir: &Path) -> Result<Vec<Migration>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut migrations = BTreeMap::new();
    for entry in fs::read_dir(dir).map_err(|_| "could not read migrations directory".to_string())? {
        let entry = entry.map_err(|_| "could not read migration entry".to_string())?;
        let path = entry.path();
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            return Err("migration file name must be UTF-8".to_string());
        };
        if !name.ends_with(".sql") {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| "could not inspect migration file".to_string())?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > MAX_MIGRATION_BYTES
        {
            return Err(format!(
                "migration `{name}` must be a regular SQL file no larger than 4 MiB"
            ));
        }
        let (version, label) = name
            .strip_suffix(".sql")
            .and_then(|stem| stem.split_once('_'))
            .ok_or_else(|| format!("invalid migration file name `{name}`"))?;
        if version.len() < 4
            || !version.bytes().all(|byte| byte.is_ascii_digit())
            || label.is_empty()
            || !label
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_lowercase())
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(format!("invalid migration file name `{name}`"));
        }
        let version = version
            .parse::<i64>()
            .map_err(|_| format!("migration version is out of range in `{name}`"))?;
        if version == 0 {
            return Err(format!("migration version must be positive in `{name}`"));
        }
        let sql = fs::read_to_string(path)
            .map_err(|_| format!("could not read migration `{name}` as UTF-8"))?;
        let checksum = Sha256::digest(sql.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if migrations
            .insert(
                version,
                Migration {
                    version,
                    name: name.to_string(),
                    sql,
                    checksum,
                },
            )
            .is_some()
        {
            return Err(format!("duplicate migration version {version}"));
        }
    }
    Ok(migrations.into_values().collect())
}

fn write_new_file_atomically(path: &Path, contents: &[u8], kind: &str) -> Result<(), String> {
    use std::io::Write;

    let parent = path
        .parent()
        .ok_or_else(|| format!("could not resolve {kind} directory"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("could not resolve {kind} file name"))?
        .to_string_lossy();
    let mut temp_path = None;
    let mut temp_file = None;
    for _ in 0..16 {
        let sequence = NEXT_ARTIFACT_TEMP.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temp_path = Some(candidate);
                temp_file = Some(file);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(format!("could not create temporary {kind} file")),
        }
    }
    let temp_path = temp_path.ok_or_else(|| format!("could not reserve temporary {kind} file"))?;
    let mut temp_file = temp_file.expect("temporary path and file are set together");
    let write_result = temp_file
        .write_all(contents)
        .and_then(|()| temp_file.sync_all());
    drop(temp_file);
    let result = write_result
        .map_err(|_| format!("could not write {kind} file"))
        .and_then(|()| {
            fs::hard_link(&temp_path, path).map_err(|_| format!("could not create {kind} file"))
        });
    let _ = fs::remove_file(temp_path);
    result
}

pub(super) fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        || !name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
    {
        return Err("name must start with a lowercase letter and contain only lowercase letters, digits, or underscores".to_string());
    }
    Ok(())
}

fn load_seeds(dir: &Path) -> Result<Vec<Seed>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut seeds = BTreeMap::new();
    for entry in fs::read_dir(dir).map_err(|_| "could not read seeds directory".to_string())? {
        let entry = entry.map_err(|_| "could not read seed entry".to_string())?;
        let path = entry.path();
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            return Err("seed file name must be UTF-8".to_string());
        };
        if !file_name.ends_with(".tyr") {
            continue;
        }
        let name = file_name.strip_suffix(".tyr").unwrap_or_default();
        validate_name(name).map_err(|_| format!("invalid seed file name `{file_name}`"))?;
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| "could not inspect seed file".to_string())?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > MAX_MIGRATION_BYTES
        {
            return Err(format!(
                "seed `{file_name}` must be a regular Tyrus file no larger than 4 MiB"
            ));
        }
        let source = fs::read(path).map_err(|_| format!("could not read seed `{file_name}`"))?;
        let checksum = Sha256::digest(&source)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        std::str::from_utf8(&source)
            .map_err(|_| format!("seed `{file_name}` must contain UTF-8 source"))?;
        seeds.insert(
            name.to_string(),
            Seed {
                name: name.to_string(),
                checksum,
            },
        );
    }
    Ok(seeds.into_values().collect())
}

fn run_seed_async(project_root: &Path, selected_seed: Option<&str>) -> Result<(), String> {
    if let Some(name) = selected_seed {
        validate_name(name)?;
    }
    let database_config = load_database_config(project_root)?;
    match database_config.backend {
        tyrus_core::db::DatabaseBackend::Postgresql | tyrus_core::db::DatabaseBackend::Mysql => {
            return run_server_seed_async(project_root, database_config, selected_seed);
        }
        tyrus_core::db::DatabaseBackend::Mongodb => {
            let seeds = load_seeds(&seed_dir(project_root)?)?;
            validate_selected_seed(&seeds, selected_seed)?;
            return mongodb_seeds::status(project_root, &database_config, &seeds, selected_seed);
        }
        tyrus_core::db::DatabaseBackend::Sqlite => {}
    }
    let (_, database_path, _) = resolve_config(project_root)?;
    if database_path == Path::new(":memory:") {
        return Err("seeds require a persistent SQLite database".to_string());
    }
    let seeds = load_seeds(&seed_dir(project_root)?)?;
    validate_selected_seed(&seeds, selected_seed)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not initialize SQLite seeds".to_string())?;
    runtime.block_on(async move { seed_status(&database_path, &seeds, selected_seed).await })
}

fn run_server_seed_async(
    project_root: &Path,
    database_config: tyrus_core::db::DatabaseConfig,
    selected_seed: Option<&str>,
) -> Result<(), String> {
    let backend = match database_config.backend {
        tyrus_core::db::DatabaseBackend::Postgresql => ServerMigrationBackend::Postgresql,
        tyrus_core::db::DatabaseBackend::Mysql => ServerMigrationBackend::Mysql,
        _ => unreachable!(),
    };
    let seeds = load_seeds(&seed_dir(project_root)?)?;
    validate_selected_seed(&seeds, selected_seed)?;
    let connection_string =
        zeroize::Zeroizing::new(server_connection_string(project_root, &database_config)?);
    let runtime_target: serde_json::Value =
        serde_json::from_str(&database_config.runtime_target(project_root)?)
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
    for (label, path) in [
        ("CA", ca_file.as_deref()),
        ("client certificate", cert_file.as_deref()),
        ("client key", key_file.as_deref()),
    ] {
        if let Some(path) = path
            && !fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
        {
            return Err(format!(
                "configured database {label} path is not a readable file"
            ));
        }
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not initialize server database seeds".to_string())?;
    runtime.block_on(async move {
        let pool = open_server_migration_pool(
            backend,
            &connection_string,
            database_config.pool.as_ref(),
            ca_file.as_deref(),
            cert_file.as_deref(),
            key_file.as_deref(),
        )
        .await?;
        let result = server_seed_status(&pool, backend, &seeds, selected_seed).await;
        pool.close().await;
        result
    })
}

fn validate_selected_seed(seeds: &[Seed], selected_seed: Option<&str>) -> Result<(), String> {
    if let Some(name) = selected_seed
        && !seeds.iter().any(|seed| seed.name == name)
    {
        return Err(format!("seed `{name}` was not found in db/seeds"));
    }
    Ok(())
}

async fn seed_history(
    connection: &mut SqliteConnection,
) -> Result<BTreeMap<String, String>, String> {
    let exists: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = ?")
            .bind(SEED_HISTORY_TABLE)
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| "could not inspect seed history".to_string())?;
    if exists == 0 {
        return Ok(BTreeMap::new());
    }
    let rows = sqlx::query("SELECT name, checksum FROM _tyrus_seeds ORDER BY name")
        .fetch_all(&mut *connection)
        .await
        .map_err(|_| "could not read seed history".to_string())?;
    let mut applied = BTreeMap::new();
    for row in rows {
        let name: String = row
            .try_get("name")
            .map_err(|_| "invalid seed history name".to_string())?;
        let checksum: String = row
            .try_get("checksum")
            .map_err(|_| "invalid seed history checksum".to_string())?;
        applied.insert(name, checksum);
    }
    Ok(applied)
}

fn verify_seed_history(seeds: &[Seed], applied: &BTreeMap<String, String>) -> Result<(), String> {
    let known = seeds
        .iter()
        .map(|seed| seed.name.as_str())
        .collect::<BTreeSet<_>>();
    for name in applied.keys() {
        if !known.contains(name.as_str()) {
            return Err(format!("applied seed `{name}` is missing from db/seeds"));
        }
    }
    for seed in seeds {
        if applied
            .get(&seed.name)
            .is_some_and(|checksum| checksum.starts_with("started:"))
        {
            return Err(format!(
                "seed '{}' is in an incomplete state; inspect the database before retrying",
                seed.name
            ));
        }
        if applied
            .get(&seed.name)
            .is_some_and(|checksum| checksum != &seed.checksum)
        {
            return Err(format!(
                "applied seed `{}` has changed; restore the original file and add a new seed",
                seed.name
            ));
        }
    }
    Ok(())
}

async fn seed_status(
    path: &Path,
    seeds: &[Seed],
    selected_seed: Option<&str>,
) -> Result<(), String> {
    let applied = if path.exists() {
        let mut connection = SqliteConnection::connect_with(&connect_options(path, false))
            .await
            .map_err(|_| "could not open SQLite database in read-only mode".to_string())?;
        seed_history(&mut connection).await?
    } else {
        BTreeMap::new()
    };
    verify_seed_history(seeds, &applied)?;
    let displayed = seeds
        .iter()
        .filter(|seed| selected_seed.is_none_or(|name| seed.name == name))
        .collect::<Vec<_>>();
    for seed in &displayed {
        let state = if applied.contains_key(&seed.name) {
            "applied"
        } else {
            "pending"
        };
        println!("{state}: {}", seed.name);
    }
    let displayed_applied = displayed
        .iter()
        .filter(|seed| applied.contains_key(&seed.name))
        .count();
    println!(
        "{} applied, {} pending",
        displayed_applied,
        displayed.len() - displayed_applied
    );
    Ok(())
}

async fn server_seed_status(
    pool: &Pool<Any>,
    backend: ServerMigrationBackend,
    seeds: &[Seed],
    selected_seed: Option<&str>,
) -> Result<(), String> {
    let exists_sql = match backend {
        ServerMigrationBackend::Postgresql => {
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = current_schema() AND table_name = '_tyrus_seeds'"
        }
        ServerMigrationBackend::Mysql => {
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = database() AND table_name = '_tyrus_seeds'"
        }
    };
    let exists: i64 = sqlx::query_scalar(exists_sql)
        .fetch_one(pool)
        .await
        .map_err(|_| "could not inspect server seed history".to_string())?;
    let applied = if exists == 0 {
        BTreeMap::new()
    } else {
        server_seed_history(pool, backend).await?
    };
    let checksums = applied
        .iter()
        .map(|(name, (checksum, _))| (name.clone(), checksum.clone()))
        .collect::<BTreeMap<_, _>>();
    verify_seed_history(seeds, &checksums)?;
    let displayed = seeds
        .iter()
        .filter(|seed| selected_seed.is_none_or(|name| seed.name == name))
        .collect::<Vec<_>>();
    for seed in &displayed {
        let state = if applied.contains_key(&seed.name) {
            "applied"
        } else {
            "pending"
        };
        println!("{state}: {}", seed.name);
    }
    println!(
        "{} applied, {} pending",
        displayed
            .iter()
            .filter(|seed| applied.contains_key(&seed.name))
            .count(),
        displayed
            .iter()
            .filter(|seed| !applied.contains_key(&seed.name))
            .count()
    );
    Ok(())
}

async fn server_seed_history(
    pool: &Pool<Any>,
    backend: ServerMigrationBackend,
) -> Result<BTreeMap<String, (String, String)>, String> {
    let rows = sqlx::query(server_seed_history_select(backend))
        .fetch_all(pool)
        .await
        .map_err(|_| "could not read server seed history".to_string())?;
    let mut applied = BTreeMap::new();
    for row in rows {
        let name: String = row
            .try_get("name")
            .map_err(|_| "invalid server seed history".to_string())?;
        let checksum: String = row
            .try_get("checksum")
            .map_err(|_| "invalid server seed history".to_string())?;
        let state: String = row
            .try_get("state")
            .map_err(|_| "invalid server seed history".to_string())?;
        if state != "applied" {
            return Err(format!(
                "seed `{name}` is in an incomplete state; inspect the database and seed file before retrying"
            ));
        }
        if applied.insert(name, (checksum, state)).is_some() {
            return Err("server seed history contains duplicate names".to_string());
        }
    }
    Ok(applied)
}

async fn verify_server_migrations_applied(
    connection: &mut sqlx::AnyConnection,
    backend: ServerMigrationBackend,
    migrations: &[Migration],
) -> Result<(), String> {
    if migrations.is_empty() {
        return Ok(());
    }
    let exists_sql = match backend {
        ServerMigrationBackend::Postgresql => {
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = current_schema() AND table_name = '_tyrus_migrations'"
        }
        ServerMigrationBackend::Mysql => {
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = database() AND table_name = '_tyrus_migrations'"
        }
    };
    let exists: i64 = sqlx::query_scalar(exists_sql)
        .fetch_one(&mut *connection)
        .await
        .map_err(|_| "could not inspect server migration history before seeds".to_string())?;
    if exists == 0 {
        return Err("apply all pending migrations before applying seeds".to_string());
    }
    let applied = server_history_for_executor(connection, backend).await?;
    verify_history(migrations, &applied)?;
    if applied.len() != migrations.len() {
        return Err("apply all pending migrations before applying seeds".to_string());
    }
    Ok(())
}

fn validate_migration_sql(
    sql: &str,
    backend: Option<ServerMigrationBackend>,
) -> Result<bool, String> {
    let bytes = sql.as_bytes();
    let mut index = 0;
    let mut has_statement = false;
    let mut statement_start = true;
    let mut create_prefix = 0;
    let mut in_trigger = false;
    while index < bytes.len() {
        match bytes[index] {
            b'$' if backend == Some(ServerMigrationBackend::Postgresql) => {
                if let Some(delimiter_end) = postgres_dollar_delimiter_end(bytes, index) {
                    let delimiter = &bytes[index..delimiter_end];
                    let body_start = delimiter_end;
                    let body_end = bytes[body_start..]
                        .windows(delimiter.len())
                        .position(|window| window == delimiter)
                        .map(|offset| body_start + offset)
                        .ok_or_else(|| {
                            "unterminated PostgreSQL dollar-quoted SQL text".to_string()
                        })?;
                    index = body_end + delimiter.len();
                    has_statement = true;
                    statement_start = false;
                    create_prefix = 0;
                } else {
                    has_statement = true;
                    statement_start = false;
                    create_prefix = 0;
                    index += 1;
                }
            }
            b'\'' | b'"' | b'`' => {
                let quote = bytes[index];
                let escaped_string = backend == Some(ServerMigrationBackend::Postgresql)
                    && quote == b'\''
                    && index > 0
                    && matches!(bytes[index - 1], b'e' | b'E')
                    && (index < 2
                        || !(bytes[index - 2].is_ascii_alphanumeric()
                            || matches!(bytes[index - 2], b'_' | b'$')));
                index += 1;
                let mut closed = false;
                while index < bytes.len() {
                    if escaped_string && bytes[index] == b'\\' {
                        index = (index + 2).min(bytes.len());
                    } else if bytes[index] == quote {
                        index += 1;
                        if bytes.get(index) == Some(&quote) {
                            index += 1;
                        } else {
                            closed = true;
                            break;
                        }
                    } else {
                        index += 1;
                    }
                }
                if !closed {
                    return Err("unterminated quoted SQL text".to_string());
                }
                has_statement = true;
                statement_start = false;
                create_prefix = 0;
            }
            b'[' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b']' {
                    index += 1;
                }
                if index == bytes.len() {
                    return Err("unterminated quoted SQL identifier".to_string());
                }
                index += 1;
                has_statement = true;
                statement_start = false;
                create_prefix = 0;
            }
            b'-' if bytes.get(index + 1) == Some(&b'-') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'#' if backend == Some(ServerMigrationBackend::Mysql) => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                if backend == Some(ServerMigrationBackend::Postgresql) {
                    let mut depth = 1usize;
                    while index + 1 < bytes.len() && depth > 0 {
                        if bytes[index] == b'/' && bytes[index + 1] == b'*' {
                            depth += 1;
                            index += 2;
                        } else if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                            depth -= 1;
                            index += 2;
                        } else {
                            index += 1;
                        }
                    }
                    if depth != 0 {
                        return Err("unterminated SQL comment".to_string());
                    }
                } else {
                    while index + 1 < bytes.len()
                        && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                    {
                        index += 1;
                    }
                    if index + 1 >= bytes.len() {
                        return Err("unterminated SQL comment".to_string());
                    }
                    index += 2;
                }
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let start = index;
                index += 1;
                while bytes
                    .get(index)
                    .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                {
                    index += 1;
                }
                let word = &sql[start..index];
                if statement_start {
                    if in_trigger && word.eq_ignore_ascii_case("END") {
                        in_trigger = false;
                    } else if [
                        "BEGIN",
                        "COMMIT",
                        "END",
                        "ROLLBACK",
                        "SAVEPOINT",
                        "RELEASE",
                        "ATTACH",
                        "DETACH",
                        "VACUUM",
                        "PRAGMA",
                    ]
                    .iter()
                    .any(|forbidden| word.eq_ignore_ascii_case(forbidden))
                    {
                        return Err(format!("`{word}` is not allowed in a managed migration"));
                    }
                    create_prefix = if word.eq_ignore_ascii_case("CREATE") {
                        1
                    } else {
                        0
                    };
                } else if create_prefix == 1
                    && ["TEMP", "TEMPORARY"]
                        .iter()
                        .any(|candidate| word.eq_ignore_ascii_case(candidate))
                {
                    create_prefix = 2;
                } else if create_prefix != 0 {
                    in_trigger = word.eq_ignore_ascii_case("TRIGGER");
                    create_prefix = 0;
                }
                has_statement = true;
                statement_start = false;
            }
            b';' => {
                statement_start = true;
                create_prefix = 0;
                index += 1;
            }
            byte => {
                if !byte.is_ascii_whitespace() {
                    has_statement = true;
                    statement_start = false;
                    create_prefix = 0;
                }
                index += 1;
            }
        }
    }
    Ok(has_statement)
}

fn postgres_dollar_delimiter_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'$') {
        return None;
    }
    let mut index = start + 1;
    if bytes.get(index) != Some(&b'$') {
        if !bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        {
            return None;
        }
        index += 1;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            index += 1;
        }
    }
    (bytes.get(index) == Some(&b'$')).then_some(index + 1)
}

fn run_async(project_root: &Path, apply: bool) -> Result<(), String> {
    let database_config = load_database_config(project_root)?;
    match database_config.backend {
        tyrus_core::db::DatabaseBackend::Postgresql | tyrus_core::db::DatabaseBackend::Mysql => {
            let migrations = load_migrations(&migration_dir(project_root)?)?;
            if apply && migrations.is_empty() {
                return Err("no migrations found in db/migrations".to_string());
            }
            let connection_string =
                zeroize::Zeroizing::new(server_connection_string(project_root, &database_config)?);
            let pool_options = database_config.pool.as_ref();
            let runtime_target: serde_json::Value =
                serde_json::from_str(&database_config.runtime_target(project_root)?)
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
            for (label, path) in [
                ("CA", ca_file.as_deref()),
                ("client certificate", cert_file.as_deref()),
                ("client key", key_file.as_deref()),
            ] {
                if let Some(path) = path
                    && !fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
                {
                    return Err(format!(
                        "configured database {label} path is not a readable file"
                    ));
                }
            }
            let backend = match database_config.backend {
                tyrus_core::db::DatabaseBackend::Postgresql => ServerMigrationBackend::Postgresql,
                tyrus_core::db::DatabaseBackend::Mysql => ServerMigrationBackend::Mysql,
                _ => unreachable!(),
            };
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| "could not initialize server database migrations".to_string())?;
            return runtime.block_on(async move {
                let opened = open_server_migration_pool(
                    backend,
                    &connection_string,
                    pool_options,
                    ca_file.as_deref(),
                    cert_file.as_deref(),
                    key_file.as_deref(),
                )
                .await;
                let pool = opened?;
                if apply {
                    apply_server_migrations(&pool, backend, &migrations).await
                } else {
                    server_status(&pool, backend, &migrations).await
                }
            });
        }
        tyrus_core::db::DatabaseBackend::Mongodb => {
            return Err("migrations are not available for MongoDB".to_string());
        }
        tyrus_core::db::DatabaseBackend::Sqlite => {}
    }
    let (config, database_path, _) = resolve_config(project_root)?;
    if database_path == Path::new(":memory:") {
        return Err("migrations require a persistent SQLite database".to_string());
    }
    let migrations = load_migrations(&migration_dir(project_root)?)?;
    if apply && migrations.is_empty() {
        return Err("no migrations found in db/migrations".to_string());
    }
    if apply {
        let parent = database_path
            .parent()
            .ok_or_else(|| "database path has no parent directory".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|_| "could not create SQLite database directory".to_string())?;
        if config.resolve_database_path(project_root)? != database_path {
            return Err("database path changed while creating its directory".to_string());
        }
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not initialize SQLite migrations".to_string())?;
    runtime.block_on(async move {
        if apply {
            apply_migrations(&database_path, &migrations).await
        } else {
            status(&database_path, &migrations).await
        }
    })
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum ServerMigrationBackend {
    Postgresql,
    Mysql,
}

pub(super) fn server_connection_string(
    project_root: &Path,
    database: &tyrus_core::db::DatabaseConfig,
) -> Result<String, String> {
    let project = tyrus_core::project::load_project(project_root)
        .map_err(|_| "could not load project environment configuration".to_string())?;
    let name = database
        .connection_env
        .as_deref()
        .ok_or_else(|| "server database requires database.connectionEnv".to_string())?;
    if let Ok(value) = std::env::var(name)
        && !value.is_empty()
    {
        return Ok(value);
    }
    if project
        .security
        .as_ref()
        .is_some_and(|security| security.env_sealed)
    {
        return Err(
            "database connection variable is missing from the process environment".to_string(),
        );
    }
    let schema = tyrus_core::env::load_env_schema(project_root, &project)
        .map_err(|_| "could not load the configured environment file".to_string())?;
    schema
        .and_then(|(schema, _)| schema.var(name).map(|variable| variable.raw_value.clone()))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "database connection variable is not set".to_string())
}

pub(super) async fn open_server_migration_pool(
    backend: ServerMigrationBackend,
    connection_string: &str,
    pool_config: Option<&tyrus_core::db::DatabasePoolConfig>,
    ca_file: Option<&str>,
    cert_file: Option<&str>,
    key_file: Option<&str>,
) -> Result<Pool<Any>, String> {
    if cert_file.is_some() != key_file.is_some() {
        return Err(
            "database client TLS certificate and key must be configured together".to_string(),
        );
    }
    for path in [ca_file, cert_file, key_file].into_iter().flatten() {
        if !fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
            return Err("configured database TLS path is not a readable file".to_string());
        }
    }
    let mut connect_options = connection_string
        .parse::<AnyConnectOptions>()
        .map_err(|_| "database connection variable is not a valid server URL".to_string())?;
    let url = &mut connect_options.database_url;
    let valid_scheme = match backend {
        ServerMigrationBackend::Postgresql => matches!(url.scheme(), "postgres" | "postgresql"),
        ServerMigrationBackend::Mysql => url.scheme() == "mysql",
    };
    if !valid_scheme || !url.host_str().is_some_and(|host| !host.is_empty()) {
        return Err(
            "database connection variable is not a valid URL for the configured backend"
                .to_string(),
        );
    }
    tls::configure_server_url(backend, &mut connect_options, ca_file, cert_file, key_file)?;
    sqlx::any::install_default_drivers();
    let connect_timeout = pool_config
        .and_then(|value| value.connect_timeout_ms)
        .unwrap_or(10_000);
    let max_connections = pool_config
        .and_then(|value| value.max_connections)
        .unwrap_or(10);
    let acquire_timeout = pool_config
        .and_then(|value| value.acquire_timeout_ms)
        .unwrap_or(5_000);
    let idle_timeout = pool_config
        .and_then(|value| value.idle_timeout_ms)
        .unwrap_or(30_000);
    let max_lifetime = pool_config
        .and_then(|value| value.max_lifetime_ms)
        .unwrap_or(1_800_000);
    tokio::time::timeout(
        Duration::from_millis(connect_timeout),
        AnyPoolOptions::new()
            .min_connections(0)
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_millis(acquire_timeout))
            .idle_timeout(Some(Duration::from_millis(idle_timeout)))
            .max_lifetime(Some(Duration::from_millis(max_lifetime)))
            .connect_with(connect_options),
    )
    .await
    .map_err(|_| "database connection timed out".to_string())?
    .map_err(|_| "could not connect to the configured server database".to_string())
}

async fn server_status(
    pool: &Pool<Any>,
    backend: ServerMigrationBackend,
    migrations: &[Migration],
) -> Result<(), String> {
    let exists_sql = match backend {
        ServerMigrationBackend::Postgresql => {
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = current_schema() AND table_name = '_tyrus_migrations'"
        }
        ServerMigrationBackend::Mysql => {
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = database() AND table_name = '_tyrus_migrations'"
        }
    };
    let exists: i64 = sqlx::query_scalar(exists_sql)
        .fetch_one(pool)
        .await
        .map_err(|_| "could not inspect server migration history".to_string())?;
    let applied = if exists == 0 {
        BTreeMap::new()
    } else {
        server_history(pool, backend).await?
    };
    verify_history(migrations, &applied)?;
    for migration in migrations {
        let state = if applied.contains_key(&migration.version) {
            "applied"
        } else {
            "pending"
        };
        println!("{state}: {}", migration.name);
    }
    println!(
        "{} applied, {} pending",
        applied.len(),
        migrations.len() - applied.len()
    );
    Ok(())
}

async fn server_history(
    pool: &Pool<Any>,
    backend: ServerMigrationBackend,
) -> Result<BTreeMap<i64, (String, String)>, String> {
    let has_state = server_migration_history_has_state_pool(pool, backend).await?;
    let rows = sqlx::query(server_migration_history_select(backend, has_state))
        .fetch_all(pool)
        .await
        .map_err(|_| "could not read server migration history".to_string())?;
    let mut applied = BTreeMap::new();
    for row in rows {
        let version: i64 = row
            .try_get("version")
            .map_err(|_| "invalid server migration history".to_string())?;
        let name: String = row
            .try_get("name")
            .map_err(|_| "invalid server migration history".to_string())?;
        let checksum: String = row
            .try_get("checksum")
            .map_err(|_| "invalid server migration history".to_string())?;
        let state = if has_state {
            row.try_get("state")
                .map_err(|_| "invalid server migration history".to_string())?
        } else {
            "applied".to_string()
        };
        if state != "applied" {
            return Err(format!(
                "migration `{name}` is in an incomplete state; inspect the database and migration file before retrying"
            ));
        }
        if applied.insert(version, (name, checksum)).is_some() {
            return Err("server migration history contains duplicate versions".to_string());
        }
    }
    Ok(applied)
}

async fn apply_server_migrations(
    pool: &Pool<Any>,
    backend: ServerMigrationBackend,
    migrations: &[Migration],
) -> Result<(), String> {
    match backend {
        ServerMigrationBackend::Postgresql => {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|_| "could not begin server migration transaction".to_string())?;
            sqlx::query("SELECT pg_advisory_xact_lock(6172137290161)")
                .execute(&mut *transaction)
                .await
                .map_err(|_| "could not acquire PostgreSQL migration lock".to_string())?;
            let result = apply_server_locked(&mut transaction, backend, migrations, true).await;
            match result {
                Ok(completed) => {
                    transaction
                        .commit()
                        .await
                        .map_err(|_| "could not commit PostgreSQL migrations".to_string())?;
                    report_applied(&completed);
                    Ok(())
                }
                Err(error) => {
                    transaction
                        .rollback()
                        .await
                        .map_err(|_| format!("{error}; PostgreSQL rollback failed"))?;
                    Err(error)
                }
            }
        }
        ServerMigrationBackend::Mysql => {
            let mut connection = pool
                .acquire()
                .await
                .map_err(|_| "could not acquire MySQL migration connection".to_string())?;
            let lock: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK('_tyrus_migrations', 10)")
                .fetch_one(&mut *connection)
                .await
                .map_err(|_| "could not acquire MySQL migration lock".to_string())?;
            if lock != Some(1) {
                return Err("could not acquire MySQL migration lock".to_string());
            }
            let result = apply_server_locked(&mut connection, backend, migrations, false).await;
            let unlock =
                sqlx::query_scalar::<_, Option<i64>>("SELECT RELEASE_LOCK('_tyrus_migrations')")
                    .fetch_one(&mut *connection)
                    .await;
            if !matches!(unlock, Ok(Some(1))) {
                let _ = connection.close().await;
                return Err("could not release MySQL migration lock".to_string());
            }
            let completed = result?;
            report_applied(&completed);
            Ok(())
        }
    }
}

fn report_applied(completed: &[String]) {
    for name in completed {
        println!("applied: {name}");
    }
    println!("{} migration(s) applied", completed.len());
}

async fn apply_server_locked(
    connection: &mut sqlx::AnyConnection,
    backend: ServerMigrationBackend,
    migrations: &[Migration],
    transactional: bool,
) -> Result<Vec<String>, String> {
    sqlx::query("CREATE TABLE IF NOT EXISTS _tyrus_migrations (version BIGINT PRIMARY KEY, name VARCHAR(255) NOT NULL, checksum CHAR(64) NOT NULL, state VARCHAR(16) NOT NULL DEFAULT 'applied', applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP)")
        .execute(&mut *connection)
        .await
        .map_err(|_| "could not create server migration history".to_string())?;
    upgrade_server_migration_history(connection, backend).await?;
    let applied = server_history_for_executor(connection, backend).await?;
    verify_history(migrations, &applied)?;
    let mut completed = Vec::new();
    for migration in migrations {
        if applied.contains_key(&migration.version) {
            continue;
        }
        if !validate_migration_sql(&migration.sql, Some(backend))
            .map_err(|detail| format!("migration `{}`: {detail}", migration.name))?
        {
            return Err(format!(
                "migration `{}` contains no SQL statement",
                migration.name
            ));
        }
        let (insert_sql, update_sql) = match backend {
            ServerMigrationBackend::Postgresql => (
                "INSERT INTO _tyrus_migrations (version, name, checksum, state) VALUES ($1, $2, $3, 'started')",
                "UPDATE _tyrus_migrations SET state = 'applied', applied_at = CURRENT_TIMESTAMP WHERE version = $1 AND checksum = $2",
            ),
            ServerMigrationBackend::Mysql => (
                "INSERT INTO _tyrus_migrations (version, name, checksum, state) VALUES (?, ?, ?, 'started')",
                "UPDATE _tyrus_migrations SET state = 'applied', applied_at = CURRENT_TIMESTAMP WHERE version = ? AND checksum = ?",
            ),
        };
        sqlx::query(insert_sql)
            .bind(migration.version)
            .bind(&migration.name)
            .bind(&migration.checksum)
            .execute(&mut *connection)
            .await
            .map_err(|_| "could not reserve server migration history".to_string())?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(migration.sql.clone()))
            .execute(&mut *connection)
            .await
            .map_err(|_| {
                if transactional { format!("migration `{}` failed; the PostgreSQL batch will be rolled back", migration.name) }
                else { format!("migration `{}` failed and may have partially changed MySQL; inspect it before retrying", migration.name) }
            })?;
        sqlx::query(update_sql)
            .bind(migration.version)
            .bind(&migration.checksum)
            .execute(&mut *connection)
            .await
            .map_err(|_| "could not finalize server migration history".to_string())?;
        completed.push(migration.name.clone());
    }
    Ok(completed)
}

async fn server_history_for_executor(
    connection: &mut sqlx::AnyConnection,
    backend: ServerMigrationBackend,
) -> Result<BTreeMap<i64, (String, String)>, String> {
    let has_state = server_migration_history_has_state_executor(connection, backend).await?;
    let rows = sqlx::query(server_migration_history_select(backend, has_state))
        .fetch_all(&mut *connection)
        .await
        .map_err(|_| "could not read server migration history".to_string())?;
    let mut applied = BTreeMap::new();
    for row in rows {
        let version: i64 = row
            .try_get("version")
            .map_err(|_| "invalid server migration history".to_string())?;
        let name: String = row
            .try_get("name")
            .map_err(|_| "invalid server migration history".to_string())?;
        let checksum: String = row
            .try_get("checksum")
            .map_err(|_| "invalid server migration history".to_string())?;
        let state = if has_state {
            row.try_get("state")
                .map_err(|_| "invalid server migration history".to_string())?
        } else {
            "applied".to_string()
        };
        if state != "applied" {
            return Err(format!(
                "migration `{name}` is in an incomplete state; inspect the database and migration file before retrying"
            ));
        }
        if applied.insert(version, (name, checksum)).is_some() {
            return Err("server migration history contains duplicate versions".to_string());
        }
    }
    Ok(applied)
}

async fn server_migration_history_has_state_pool(
    pool: &Pool<Any>,
    backend: ServerMigrationBackend,
) -> Result<bool, String> {
    sqlx::query_scalar::<_, i64>(server_migration_state_column_query(backend))
        .fetch_one(pool)
        .await
        .map(|count| count > 0)
        .map_err(|_| "could not inspect server migration history schema".to_string())
}

async fn server_migration_history_has_state_executor(
    connection: &mut sqlx::AnyConnection,
    backend: ServerMigrationBackend,
) -> Result<bool, String> {
    sqlx::query_scalar::<_, i64>(server_migration_state_column_query(backend))
        .fetch_one(&mut *connection)
        .await
        .map(|count| count > 0)
        .map_err(|_| "could not inspect server migration history schema".to_string())
}

async fn upgrade_server_migration_history(
    connection: &mut sqlx::AnyConnection,
    backend: ServerMigrationBackend,
) -> Result<(), String> {
    if server_migration_history_has_state_executor(connection, backend).await? {
        return Ok(());
    }
    sqlx::query(SERVER_MIGRATION_HISTORY_UPGRADE_SQL)
        .execute(&mut *connection)
        .await
        .map_err(|_| "could not upgrade legacy server migration history".to_string())?;
    Ok(())
}

const SERVER_MIGRATION_HISTORY_UPGRADE_SQL: &str =
    "ALTER TABLE _tyrus_migrations ADD COLUMN state VARCHAR(16) NOT NULL DEFAULT 'applied'";

fn server_migration_state_column_query(backend: ServerMigrationBackend) -> &'static str {
    match backend {
        ServerMigrationBackend::Postgresql => {
            "SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = current_schema() AND table_name = '_tyrus_migrations' AND column_name = 'state'"
        }
        ServerMigrationBackend::Mysql => {
            "SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = database() AND table_name = '_tyrus_migrations' AND column_name = 'state'"
        }
    }
}

fn server_migration_history_select(
    backend: ServerMigrationBackend,
    has_state: bool,
) -> &'static str {
    match (backend, has_state) {
        (ServerMigrationBackend::Postgresql, true) => {
            "SELECT version, name, checksum::text AS checksum, state FROM _tyrus_migrations ORDER BY version"
        }
        (ServerMigrationBackend::Postgresql, false) => {
            "SELECT version, name, checksum::text AS checksum FROM _tyrus_migrations ORDER BY version"
        }
        (ServerMigrationBackend::Mysql, true) => {
            "SELECT version, name, checksum, state FROM _tyrus_migrations ORDER BY version"
        }
        (ServerMigrationBackend::Mysql, false) => {
            "SELECT version, name, checksum FROM _tyrus_migrations ORDER BY version"
        }
    }
}

fn server_seed_history_select(backend: ServerMigrationBackend) -> &'static str {
    match backend {
        ServerMigrationBackend::Postgresql => {
            "SELECT name, checksum::text AS checksum, state FROM _tyrus_seeds ORDER BY name"
        }
        ServerMigrationBackend::Mysql => {
            "SELECT name, checksum, state FROM _tyrus_seeds ORDER BY name"
        }
    }
}

fn server_seed_lookup_select(backend: ServerMigrationBackend) -> &'static str {
    match backend {
        ServerMigrationBackend::Postgresql => {
            "SELECT checksum::text AS checksum, state FROM _tyrus_seeds WHERE name = $1"
        }
        ServerMigrationBackend::Mysql => "SELECT checksum, state FROM _tyrus_seeds WHERE name = ?",
    }
}

fn connect_options(path: &Path, create: bool) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path)
        .read_only(!create)
        .create_if_missing(create)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
}

async fn history(
    connection: &mut SqliteConnection,
) -> Result<BTreeMap<i64, (String, String)>, String> {
    let exists: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = ?")
            .bind(HISTORY_TABLE)
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| "could not inspect migration history".to_string())?;
    if exists == 0 {
        return Ok(BTreeMap::new());
    }
    let rows =
        sqlx::query("SELECT version, name, checksum FROM _tyrus_migrations ORDER BY version")
            .fetch_all(&mut *connection)
            .await
            .map_err(|_| "could not read migration history".to_string())?;
    let mut applied = BTreeMap::new();
    for row in rows {
        let version: i64 = row
            .try_get("version")
            .map_err(|_| "invalid migration history version".to_string())?;
        let name: String = row
            .try_get("name")
            .map_err(|_| "invalid migration history name".to_string())?;
        let checksum: String = row
            .try_get("checksum")
            .map_err(|_| "invalid migration history checksum".to_string())?;
        applied.insert(version, (name, checksum));
    }
    Ok(applied)
}

fn verify_history(
    migrations: &[Migration],
    applied: &BTreeMap<i64, (String, String)>,
) -> Result<(), String> {
    let known = migrations
        .iter()
        .map(|migration| migration.version)
        .collect::<BTreeSet<_>>();
    for version in applied.keys() {
        if !known.contains(version) {
            return Err(format!(
                "applied migration {version} is missing from db/migrations"
            ));
        }
    }
    for migration in migrations {
        if let Some((name, checksum)) = applied.get(&migration.version)
            && (name != &migration.name || checksum != &migration.checksum)
        {
            return Err(format!(
                "applied migration `{}` has changed; restore the original file",
                migration.name
            ));
        }
    }
    if let Some(last) = applied.keys().next_back()
        && migrations
            .iter()
            .any(|migration| migration.version < *last && !applied.contains_key(&migration.version))
    {
        return Err("a pending migration precedes an applied migration".to_string());
    }
    Ok(())
}

async fn status(path: &Path, migrations: &[Migration]) -> Result<(), String> {
    let applied = if path.exists() {
        let mut connection = SqliteConnection::connect_with(&connect_options(path, false))
            .await
            .map_err(|_| "could not open SQLite database in read-only mode".to_string())?;
        history(&mut connection).await?
    } else {
        BTreeMap::new()
    };
    verify_history(migrations, &applied)?;
    for migration in migrations {
        let state = if applied.contains_key(&migration.version) {
            "applied"
        } else {
            "pending"
        };
        println!("{state}: {}", migration.name);
    }
    println!(
        "{} applied, {} pending",
        applied.len(),
        migrations.len() - applied.len()
    );
    Ok(())
}

async fn apply_migrations(path: &Path, migrations: &[Migration]) -> Result<(), String> {
    let mut connection = SqliteConnection::connect_with(&connect_options(path, true))
        .await
        .map_err(|_| "could not open SQLite database for migrations".to_string())?;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut connection)
        .await
        .map_err(|_| "could not acquire SQLite migration lock".to_string())?;
    let result = apply_locked(&mut connection, migrations).await;
    match result {
        Ok(applied) => {
            sqlx::query("COMMIT")
                .execute(&mut connection)
                .await
                .map_err(|_| "could not commit SQLite migrations".to_string())?;
            for name in &applied {
                println!("applied: {name}");
            }
            println!("{} migration(s) applied", applied.len());
            Ok(())
        }
        Err(error) => {
            sqlx::query("ROLLBACK")
                .execute(&mut connection)
                .await
                .map_err(|_| format!("{error}; SQLite rollback also failed"))?;
            Err(error)
        }
    }
}

async fn apply_locked(
    connection: &mut SqliteConnection,
    migrations: &[Migration],
) -> Result<Vec<String>, String> {
    sqlx::query("CREATE TABLE IF NOT EXISTS _tyrus_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, checksum TEXT NOT NULL, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)")
        .execute(&mut *connection).await
        .map_err(|_| "could not create SQLite migration history".to_string())?;
    let applied = history(connection).await?;
    verify_history(migrations, &applied)?;
    let mut completed = Vec::new();
    for migration in migrations {
        if applied.contains_key(&migration.version) {
            continue;
        }
        let has_statement = validate_migration_sql(&migration.sql, None)
            .map_err(|detail| format!("migration `{}`: {detail}", migration.name))?;
        if !has_statement {
            return Err(format!(
                "migration `{}` contains no SQL statement",
                migration.name
            ));
        }
        sqlx::raw_sql(sqlx::AssertSqlSafe(migration.sql.clone()))
            .execute(&mut *connection)
            .await
            .map_err(|_| {
                format!(
                    "migration `{}` failed; all changes will be rolled back",
                    migration.name
                )
            })?;
        sqlx::query("INSERT INTO _tyrus_migrations (version, name, checksum) VALUES (?, ?, ?)")
            .bind(migration.version)
            .bind(&migration.name)
            .bind(&migration.checksum)
            .execute(&mut *connection)
            .await
            .map_err(|_| "could not record migration history".to_string())?;
        completed.push(migration.name.clone());
    }
    Ok(completed)
}

#[cfg(test)]
mod server_migration_history_tests {
    use super::{
        SERVER_MIGRATION_HISTORY_UPGRADE_SQL, ServerMigrationBackend,
        server_migration_history_select, server_migration_state_column_query,
        server_seed_history_select, server_seed_lookup_select,
    };

    #[test]
    fn legacy_server_migration_history_is_read_without_state() {
        assert_eq!(
            server_migration_history_select(ServerMigrationBackend::Mysql, false),
            "SELECT version, name, checksum FROM _tyrus_migrations ORDER BY version"
        );
        assert_eq!(
            server_migration_history_select(ServerMigrationBackend::Mysql, true),
            "SELECT version, name, checksum, state FROM _tyrus_migrations ORDER BY version"
        );
    }

    #[test]
    fn postgres_history_queries_cast_fixed_width_checksums_to_text() {
        for query in [
            server_migration_history_select(ServerMigrationBackend::Postgresql, true),
            server_migration_history_select(ServerMigrationBackend::Postgresql, false),
            server_seed_history_select(ServerMigrationBackend::Postgresql),
            server_seed_lookup_select(ServerMigrationBackend::Postgresql),
        ] {
            assert!(query.contains("checksum::text AS checksum"));
        }
        assert!(
            !server_migration_history_select(ServerMigrationBackend::Mysql, true)
                .contains("checksum::text")
        );
        assert!(
            !server_seed_history_select(ServerMigrationBackend::Mysql).contains("checksum::text")
        );
        assert!(
            !server_seed_lookup_select(ServerMigrationBackend::Mysql).contains("checksum::text")
        );
    }

    #[test]
    fn server_migration_history_upgrade_preserves_legacy_rows_as_applied() {
        assert!(
            SERVER_MIGRATION_HISTORY_UPGRADE_SQL
                .contains("ADD COLUMN state VARCHAR(16) NOT NULL DEFAULT 'applied'")
        );
    }

    #[test]
    fn server_migration_history_state_lookup_uses_backend_catalog() {
        assert!(
            server_migration_state_column_query(ServerMigrationBackend::Postgresql)
                .contains("table_schema = current_schema()")
        );
        assert!(
            server_migration_state_column_query(ServerMigrationBackend::Mysql)
                .contains("table_schema = database()")
        );
    }
}
