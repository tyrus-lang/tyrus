use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseBackend {
    Sqlite,
    Postgresql,
    Mysql,
    Mongodb,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct DatabasePoolConfig {
    pub max_connections: Option<u32>,
    pub connect_timeout_ms: Option<u64>,
    pub acquire_timeout_ms: Option<u64>,
    pub query_timeout_ms: Option<u64>,
    pub idle_timeout_ms: Option<u64>,
    pub max_lifetime_ms: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct DatabaseTlsConfig {
    pub ca_file: Option<String>,
    pub cert_file: Option<String>,
    pub key_file: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SqliteJournalMode {
    Wal,
    Delete,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SqliteSynchronous {
    Full,
    Normal,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct DatabaseSqliteConfig {
    pub journal_mode: Option<SqliteJournalMode>,
    pub synchronous: Option<SqliteSynchronous>,
    pub busy_timeout_ms: Option<u64>,
}

impl DatabaseSqliteConfig {
    fn validate(&self) -> Result<(), String> {
        if self
            .busy_timeout_ms
            .is_some_and(|timeout| !(1..=120_000).contains(&timeout))
        {
            return Err(
                "invalid tyrus.json: `database.sqlite.busyTimeoutMs` must be between 1 and 120000"
                    .to_string(),
            );
        }
        Ok(())
    }

    fn resolve(&self) -> ResolvedDatabaseSqliteConfig {
        ResolvedDatabaseSqliteConfig {
            journal_mode: self.journal_mode.unwrap_or(SqliteJournalMode::Wal),
            synchronous: self.synchronous.unwrap_or(SqliteSynchronous::Full),
            busy_timeout_ms: self.busy_timeout_ms.unwrap_or(5_000),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedDatabaseSqliteConfig {
    journal_mode: SqliteJournalMode,
    synchronous: SqliteSynchronous,
    busy_timeout_ms: u64,
}

impl Default for ResolvedDatabaseSqliteConfig {
    fn default() -> Self {
        DatabaseSqliteConfig::default().resolve()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedDatabasePoolConfig {
    max_connections: u32,
    connect_timeout_ms: u64,
    acquire_timeout_ms: u64,
    query_timeout_ms: u64,
    idle_timeout_ms: u64,
    max_lifetime_ms: u64,
}

impl Default for ResolvedDatabasePoolConfig {
    fn default() -> Self {
        DatabasePoolConfig::default().resolve()
    }
}

impl DatabasePoolConfig {
    fn validate(&self) -> Result<(), String> {
        validate_pool_value("maxConnections", self.max_connections, 1, 100)?;
        validate_pool_value("connectTimeoutMs", self.connect_timeout_ms, 1, 120_000)?;
        validate_pool_value("acquireTimeoutMs", self.acquire_timeout_ms, 1, 120_000)?;
        validate_pool_value("queryTimeoutMs", self.query_timeout_ms, 1, 120_000)?;
        validate_pool_value("idleTimeoutMs", self.idle_timeout_ms, 1, 86_400_000)?;
        validate_pool_value("maxLifetimeMs", self.max_lifetime_ms, 1, 86_400_000)?;
        Ok(())
    }

    fn resolve(&self) -> ResolvedDatabasePoolConfig {
        ResolvedDatabasePoolConfig {
            max_connections: self.max_connections.unwrap_or(10),
            connect_timeout_ms: self.connect_timeout_ms.unwrap_or(10_000),
            acquire_timeout_ms: self.acquire_timeout_ms.unwrap_or(5_000),
            query_timeout_ms: self.query_timeout_ms.unwrap_or(30_000),
            idle_timeout_ms: self.idle_timeout_ms.unwrap_or(30_000),
            max_lifetime_ms: self.max_lifetime_ms.unwrap_or(1_800_000),
        }
    }
}

fn validate_pool_value<T: Copy + PartialOrd + std::fmt::Display>(
    name: &str,
    value: Option<T>,
    minimum: T,
    maximum: T,
) -> Result<(), String> {
    if value.is_some_and(|value| value < minimum || value > maximum) {
        return Err(format!(
            "invalid tyrus.json: `database.pool.{name}` must be between {minimum} and {maximum}"
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    pub backend: DatabaseBackend,
    #[serde(default)]
    pub path: String,
    #[serde(default, rename = "connectionEnv")]
    pub connection_env: Option<String>,
    #[serde(default)]
    pub pool: Option<DatabasePoolConfig>,
    #[serde(default)]
    pub tls: Option<DatabaseTlsConfig>,
    #[serde(default)]
    pub sqlite: Option<DatabaseSqliteConfig>,
    #[serde(default = "default_schema_path")]
    pub schema: String,
}

impl DatabaseConfig {
    pub fn validate(&self) -> Result<(), String> {
        match self.backend {
            DatabaseBackend::Sqlite => {
                if self.pool.is_some() {
                    return Err(
                        "invalid tyrus.json: `database.pool` is only valid for server databases"
                            .to_string(),
                    );
                }
                if self.connection_env.is_some() {
                    return Err(
                        "invalid tyrus.json: `database.connectionEnv` is only valid for server databases"
                            .to_string(),
                    );
                }
                if self.tls.is_some() {
                    return Err(
                        "invalid tyrus.json: `database.tls` is only valid for server databases"
                            .to_string(),
                    );
                }
                if let Some(sqlite) = &self.sqlite {
                    sqlite.validate()?;
                }
                validate_project_relative_path(&self.path, true, "database.path")?;
            }
            DatabaseBackend::Postgresql | DatabaseBackend::Mysql | DatabaseBackend::Mongodb => {
                if self.sqlite.is_some() {
                    return Err(
                        "invalid tyrus.json: `database.sqlite` is only valid for SQLite"
                            .to_string(),
                    );
                }
                if !self.path.is_empty() {
                    return Err(
                        "invalid tyrus.json: `database.path` is only valid for SQLite".to_string(),
                    );
                }
                let name = self.connection_env.as_deref().ok_or_else(|| {
                    "invalid tyrus.json: server databases require `database.connectionEnv` to name a connection variable".to_string()
                })?;
                if !is_environment_variable_name(name) {
                    return Err("invalid tyrus.json: `database.connectionEnv` must be a valid environment variable name".to_string());
                }
                if let Some(pool) = &self.pool {
                    pool.validate()?;
                    if self.backend == DatabaseBackend::Mongodb && pool.max_lifetime_ms.is_some() {
                        return Err("invalid tyrus.json: `database.pool.maxLifetimeMs` is not supported for MongoDB".to_string());
                    }
                }
                if let Some(tls) = &self.tls {
                    if tls.ca_file.is_none() && tls.cert_file.is_none() && tls.key_file.is_none() {
                        return Err("invalid tyrus.json: `database.tls` must configure `caFile` or a client certificate and key".to_string());
                    }
                    if let Some(path) = tls.ca_file.as_deref() {
                        validate_project_relative_path(path, false, "database.tls.caFile")?;
                    }
                    match (tls.cert_file.as_deref(), tls.key_file.as_deref()) {
                        (Some(cert), Some(key)) => {
                            validate_project_relative_path(cert, false, "database.tls.certFile")?;
                            validate_project_relative_path(key, false, "database.tls.keyFile")?;
                        }
                        (Some(_), None) => return Err("invalid tyrus.json: `database.tls.keyFile` is required when `database.tls.certFile` is set".to_string()),
                        (None, Some(_)) => return Err("invalid tyrus.json: `database.tls.certFile` is required when `database.tls.keyFile` is set".to_string()),
                        (None, None) => {}
                    }
                }
            }
        }
        validate_project_relative_path(&self.schema, false, "database.schema")?;
        Ok(())
    }

    pub fn resolve_database_path(&self, project_root: &Path) -> Result<PathBuf, String> {
        self.validate()?;
        if self.backend != DatabaseBackend::Sqlite {
            return Err("`database.path` is only available for the SQLite backend".to_string());
        }
        if self.path == ":memory:" {
            return Ok(PathBuf::from(":memory:"));
        }
        resolve_confined_path(project_root, Path::new(&self.path), "database.path")
    }

    pub fn runtime_target(&self, project_root: &Path) -> Result<String, String> {
        self.validate()?;
        let pool = self
            .pool
            .as_ref()
            .map(DatabasePoolConfig::resolve)
            .unwrap_or_default();
        let target = match self.backend {
            DatabaseBackend::Sqlite => serde_json::json!({
                "backend": "sqlite",
                "path": self.resolve_database_path(project_root)?.to_string_lossy(),
                "sqlite": self.sqlite.as_ref().map(DatabaseSqliteConfig::resolve).unwrap_or_default(),
            }),
            DatabaseBackend::Postgresql => serde_json::json!({
                "backend": "postgresql",
                "connectionEnv": self.connection_env.as_deref(),
                "pool": pool,
                "tls": self.resolve_tls_target(project_root)?,
            }),
            DatabaseBackend::Mysql => serde_json::json!({
                "backend": "mysql",
                "connectionEnv": self.connection_env.as_deref(),
                "pool": pool,
                "tls": self.resolve_tls_target(project_root)?,
            }),
            DatabaseBackend::Mongodb => serde_json::json!({
                "backend": "mongodb",
                "connectionEnv": self.connection_env.as_deref(),
                "pool": {
                    "maxConnections": pool.max_connections,
                    "connectTimeoutMs": pool.connect_timeout_ms,
                    "acquireTimeoutMs": pool.acquire_timeout_ms,
                    "queryTimeoutMs": pool.query_timeout_ms,
                    "idleTimeoutMs": pool.idle_timeout_ms,
                },
                "tls": self.resolve_tls_target(project_root)?,
            }),
        };
        serde_json::to_string(&target)
            .map_err(|_| "could not encode the database runtime target".to_string())
    }

    fn resolve_tls_target(&self, project_root: &Path) -> Result<serde_json::Value, String> {
        let ca_file = self
            .tls
            .as_ref()
            .and_then(|tls| tls.ca_file.as_deref())
            .map(|path| {
                resolve_confined_path(project_root, Path::new(path), "database.tls.caFile")
                    .map(|resolved| resolved.to_string_lossy().into_owned())
            })
            .transpose()?;
        let cert_file = self
            .tls
            .as_ref()
            .and_then(|tls| tls.cert_file.as_deref())
            .map(|path| {
                resolve_confined_path(project_root, Path::new(path), "database.tls.certFile")
                    .map(|resolved| resolved.to_string_lossy().into_owned())
            })
            .transpose()?;
        let key_file = self
            .tls
            .as_ref()
            .and_then(|tls| tls.key_file.as_deref())
            .map(|path| {
                resolve_confined_path(project_root, Path::new(path), "database.tls.keyFile")
                    .map(|resolved| resolved.to_string_lossy().into_owned())
            })
            .transpose()?;
        Ok(serde_json::json!({
            "caFile": ca_file,
            "certFile": cert_file,
            "keyFile": key_file,
        }))
    }

    pub fn resolve_schema_path(&self, project_root: &Path) -> Result<PathBuf, String> {
        self.validate()?;
        resolve_confined_path(project_root, Path::new(&self.schema), "database.schema")
    }
}

fn is_environment_variable_name(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('_' | 'A'..='Z' | 'a'..='z'))
        && chars.all(|character| matches!(character, '_' | 'A'..='Z' | 'a'..='z' | '0'..='9'))
}

pub fn validate_project_relative_path(
    value: &str,
    allow_memory: bool,
    field: &str,
) -> Result<(), String> {
    if allow_memory && value == ":memory:" {
        return Ok(());
    }
    if value.is_empty()
        || value.contains('\\')
        || value.contains('?')
        || value.contains('#')
        || value.contains("://")
        || value.starts_with("//")
        || Path::new(value).is_absolute()
        || Path::new(value).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "invalid tyrus.json: `{field}` must be a project-relative path without parent traversal or URL credentials"
        ));
    }
    Ok(())
}

fn resolve_confined_path(root: &Path, relative: &Path, field: &str) -> Result<PathBuf, String> {
    let canonical_root = root.canonicalize().map_err(|_| {
        format!("could not resolve `{field}` because the project root is unavailable")
    })?;
    let candidate = canonical_root.join(relative);
    let mut existing = candidate.as_path();
    let mut missing = Vec::new();
    while fs::symlink_metadata(existing).is_err() {
        missing.push(
            existing
                .file_name()
                .ok_or_else(|| format!("invalid `{field}` path"))?
                .to_os_string(),
        );
        existing = existing
            .parent()
            .ok_or_else(|| format!("invalid `{field}` path"))?;
    }
    let mut canonical_candidate = existing
        .canonicalize()
        .map_err(|_| format!("could not resolve `{field}` inside the project"))?;
    for component in missing.into_iter().rev() {
        canonical_candidate.push(component);
    }
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(format!("`{field}` resolves outside the project root"));
    }
    Ok(canonical_candidate)
}

fn default_schema_path() -> String {
    "tyrus-db-schema.json".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    fn temp_root() -> PathBuf {
        let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("tyrus-db-config-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn deserializes_sqlite_configuration_without_credentials() {
        let config: DatabaseConfig =
            serde_json::from_str(r#"{"backend":"sqlite","path":"./data/app.db"}"#).unwrap();
        assert_eq!(config.schema, "tyrus-db-schema.json");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rejects_credential_fields_without_echoing_secret_values() {
        let raw = r#"{"backend":"sqlite","path":"./app.db","password":"ultra-secret"}"#;
        let error = serde_json::from_str::<DatabaseConfig>(raw)
            .unwrap_err()
            .to_string();
        assert!(error.contains("unknown field `password`"));
        assert!(!error.contains("ultra-secret"));
    }

    #[test]
    fn rejects_absolute_url_and_parent_paths_without_echoing_the_path() {
        for path in [
            "/etc/passwd",
            "../outside.db",
            "sqlite://user:secret@host/db",
        ] {
            let error = DatabaseConfig {
                backend: DatabaseBackend::Sqlite,
                path: path.to_string(),
                connection_env: None,
                pool: None,
                tls: None,
                sqlite: None,
                schema: "tyrus-db-schema.json".to_string(),
            }
            .validate()
            .unwrap_err();
            assert!(error.contains("database.path"));
            assert!(!error.contains(path));
            assert!(!error.contains("secret"));
        }
    }

    #[test]
    fn resolves_only_paths_confined_to_the_project_even_through_symlinks() {
        let root = temp_root();
        fs::create_dir(root.join("data")).unwrap();
        let config = DatabaseConfig {
            backend: DatabaseBackend::Sqlite,
            path: "data/app.db".to_string(),
            connection_env: None,
            pool: None,
            tls: None,
            sqlite: None,
            schema: "tyrus-db-schema.json".to_string(),
        };
        assert_eq!(
            config.resolve_database_path(&root).unwrap(),
            root.canonicalize().unwrap().join("data/app.db")
        );
        assert!(config.resolve_schema_path(&root).is_ok());

        let outside = root.with_extension("outside");
        fs::create_dir_all(&outside).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, root.join("escape")).unwrap();
            let escaping = DatabaseConfig {
                path: "escape/app.db".to_string(),
                ..config
            };
            assert!(escaping.resolve_database_path(&root).is_err());
        }
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn allows_the_explicit_in_memory_database() {
        let config = DatabaseConfig {
            backend: DatabaseBackend::Sqlite,
            path: ":memory:".to_string(),
            connection_env: None,
            pool: None,
            tls: None,
            sqlite: None,
            schema: "tyrus-db-schema.json".to_string(),
        };
        assert_eq!(
            config.resolve_database_path(Path::new(".")).unwrap(),
            PathBuf::from(":memory:")
        );
    }
}
