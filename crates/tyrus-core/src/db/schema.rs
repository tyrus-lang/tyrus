use super::config::{DatabaseBackend, DatabaseConfig, validate_project_relative_path};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub const DB_SCHEMA_FORMAT_VERSION: u32 = 1;

fn is_supported_logical_type(logical_type: &str) -> bool {
    matches!(
        logical_type,
        "boolean" | "bytes" | "f64" | "i64" | "instant" | "string" | "u64"
    )
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DbSchemaSnapshot {
    #[serde(rename = "formatVersion")]
    pub format_version: u32,
    pub backend: DatabaseBackend,
    pub source: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "schemaHash")]
    pub schema_hash: String,
    pub tables: Vec<DbTableSchema>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DbTableSchema {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    pub columns: Vec<DbColumnSchema>,
    #[serde(rename = "primaryKey")]
    pub primary_key: Vec<String>,
    pub unique: Vec<Vec<String>>,
    #[serde(rename = "foreignKeys")]
    pub foreign_keys: Vec<DbForeignKeySchema>,
    pub indexes: Vec<DbIndexSchema>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<DbTriggerSchema>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DbTriggerSchema {
    pub name: String,
    pub sql: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DbColumnSchema {
    pub name: String,
    #[serde(rename = "type")]
    pub logical_type: String,
    pub nullable: bool,
    pub ordinal: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(
        rename = "nativeType",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub native_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DbForeignKeySchema {
    pub columns: Vec<String>,
    #[serde(rename = "referencedTable")]
    pub referenced_table: String,
    #[serde(rename = "referencedColumns")]
    pub referenced_columns: Vec<String>,
    #[serde(rename = "onUpdate")]
    pub on_update: String,
    #[serde(rename = "onDelete")]
    pub on_delete: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DbIndexSchema {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub partial: bool,
    pub expression: bool,
}

#[derive(Serialize)]
struct DbSchemaHashInput<'a> {
    #[serde(rename = "formatVersion")]
    format_version: u32,
    backend: &'a DatabaseBackend,
    source: &'a str,
    #[serde(rename = "schemaVersion")]
    schema_version: i64,
    tables: &'a [DbTableSchema],
}

impl DbSchemaSnapshot {
    pub fn new(source: String, schema_version: i64, tables: Vec<DbTableSchema>) -> Self {
        Self::new_for_backend(DatabaseBackend::Sqlite, source, schema_version, tables)
    }

    pub fn new_for_backend(
        backend: DatabaseBackend,
        source: String,
        schema_version: i64,
        tables: Vec<DbTableSchema>,
    ) -> Self {
        let mut snapshot = Self {
            format_version: DB_SCHEMA_FORMAT_VERSION,
            backend,
            source,
            schema_version,
            schema_hash: String::new(),
            tables,
        };
        snapshot.schema_hash = snapshot.computed_hash();
        snapshot
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .map_err(|_| "could not read database schema snapshot".to_string())?;
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|_| "database schema snapshot is invalid JSON".to_string())?;
        let version = value
            .get("formatVersion")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                "database schema snapshot is missing integer `formatVersion`".to_string()
            })?;
        if version != u64::from(DB_SCHEMA_FORMAT_VERSION) {
            return Err(format!(
                "unsupported database schema snapshot formatVersion {version}; this compiler supports {DB_SCHEMA_FORMAT_VERSION}"
            ));
        }
        let snapshot: Self = serde_json::from_value(value)
            .map_err(|_| "database schema snapshot has an invalid shape".to_string())?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn to_pretty_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map(|text| format!("{text}\n"))
            .map_err(|_| "could not serialize database schema snapshot".to_string())
    }

    pub fn write_atomic(&self, path: &Path) -> Result<(), String> {
        let contents = self.to_pretty_json()?;
        let parent = path
            .parent()
            .ok_or_else(|| "database schema snapshot path has no parent directory".to_string())?;
        let file_name = path
            .file_name()
            .ok_or_else(|| "database schema snapshot path has no file name".to_string())?;
        static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            sequence
        ));
        fs::write(&temporary, contents)
            .map_err(|_| "could not write temporary database schema snapshot".to_string())?;
        if fs::rename(&temporary, path).is_err() {
            let _ = fs::remove_file(&temporary);
            return Err("could not replace database schema snapshot".to_string());
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != DB_SCHEMA_FORMAT_VERSION {
            return Err(format!(
                "unsupported database schema snapshot formatVersion {}",
                self.format_version
            ));
        }
        validate_project_relative_path(&self.source, true, "snapshot.source")?;
        if self.schema_version < 0 || self.schema_hash != self.computed_hash() {
            return Err(
                "database schema snapshot schemaHash does not match its contents".to_string(),
            );
        }
        let mut table_names = Vec::new();
        for table in &self.tables {
            if table.name.is_empty()
                || table_names
                    .iter()
                    .any(|name: &&str| name.eq_ignore_ascii_case(&table.name))
            {
                return Err(
                    "database schema snapshot contains an empty or duplicate table name"
                        .to_string(),
                );
            }
            table_names.push(table.name.as_str());
            let mut columns = Vec::new();
            for column in &table.columns {
                if column.name.is_empty()
                    || columns
                        .iter()
                        .any(|name: &&str| name.eq_ignore_ascii_case(&column.name))
                {
                    return Err(
                        "database schema snapshot contains an empty or duplicate column name"
                            .to_string(),
                    );
                }
                columns.push(column.name.as_str());
                if !is_supported_logical_type(&column.logical_type) {
                    return Err(format!(
                        "database schema snapshot contains unsupported logical type `{}`",
                        column.logical_type
                    ));
                }
            }
            let mut ordinals = table
                .columns
                .iter()
                .map(|column| column.ordinal)
                .collect::<Vec<_>>();
            ordinals.sort_unstable();
            if ordinals
                .iter()
                .copied()
                .enumerate()
                .any(|(index, ordinal)| ordinal as usize != index)
            {
                return Err(
                    "database schema snapshot column ordinals must be dense and zero-based"
                        .to_string(),
                );
            }
            if table
                .primary_key
                .iter()
                .any(|column| !columns.iter().any(|name| name.eq_ignore_ascii_case(column)))
            {
                return Err(format!(
                    "database schema snapshot primary key on table `{}` references an unknown column",
                    table.name
                ));
            }
            for key in &table.unique {
                if key.is_empty()
                    || key
                        .iter()
                        .any(|column| !columns.iter().any(|name| name.eq_ignore_ascii_case(column)))
                {
                    return Err(format!(
                        "database schema snapshot unique constraint on table `{}` references an unknown column in `{}`",
                        table.name,
                        key.join(", ")
                    ));
                }
            }
            for foreign_key in &table.foreign_keys {
                if foreign_key.columns.is_empty()
                    || foreign_key.columns.len() != foreign_key.referenced_columns.len()
                    || foreign_key
                        .columns
                        .iter()
                        .any(|column| !columns.iter().any(|name| name.eq_ignore_ascii_case(column)))
                    || foreign_key.referenced_table.is_empty()
                {
                    return Err(
                        "database schema snapshot contains an invalid foreign key".to_string()
                    );
                }
            }
            for index in &table.indexes {
                if index.name.is_empty()
                    || index
                        .columns
                        .iter()
                        .any(|column| !columns.iter().any(|name| name.eq_ignore_ascii_case(column)))
                {
                    return Err(
                        "database schema snapshot index references an unknown column".to_string(),
                    );
                }
            }
            let mut trigger_names = Vec::new();
            for trigger in &table.triggers {
                if trigger.name.is_empty()
                    || trigger.sql.trim().is_empty()
                    || trigger_names
                        .iter()
                        .any(|name: &&str| name.eq_ignore_ascii_case(&trigger.name))
                {
                    return Err(
                        "database schema snapshot contains an empty or duplicate trigger"
                            .to_string(),
                    );
                }
                trigger_names.push(trigger.name.as_str());
            }
        }
        let table_columns = self
            .tables
            .iter()
            .map(|table| {
                (
                    table.name.as_str(),
                    table
                        .columns
                        .iter()
                        .map(|column| column.name.as_str())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        for foreign_key in self.tables.iter().flat_map(|table| &table.foreign_keys) {
            let Some((_, referenced_columns)) = table_columns
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(&foreign_key.referenced_table))
            else {
                return Err(
                    "database schema snapshot foreign key references an unknown table".to_string(),
                );
            };
            if foreign_key.referenced_columns.iter().any(|column| {
                !referenced_columns
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(column))
            }) {
                return Err(
                    "database schema snapshot foreign key references an unknown column".to_string(),
                );
            }
        }
        Ok(())
    }

    fn computed_hash(&self) -> String {
        let input = DbSchemaHashInput {
            format_version: self.format_version,
            backend: &self.backend,
            source: &self.source,
            schema_version: self.schema_version,
            tables: &self.tables,
        };
        let bytes = serde_json::to_vec(&input).expect("schema snapshot hash input is serializable");
        let digest = sha256(&bytes);
        let mut encoded = String::with_capacity(71);
        encoded.push_str("sha256:");
        for byte in digest {
            use std::fmt::Write as _;
            let _ = write!(encoded, "{byte:02x}");
        }
        encoded
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).into()
}

#[derive(Default)]
pub struct DbSchemaCache {
    entries: HashMap<PathBuf, CachedDbSchema>,
}

struct CachedDbSchema {
    fingerprint: [u8; 32],
    snapshot: Arc<DbSchemaSnapshot>,
}

impl DbSchemaCache {
    pub fn load_configured(
        &mut self,
        project_root: &Path,
        config: &DatabaseConfig,
    ) -> Result<Arc<DbSchemaSnapshot>, String> {
        let path = config.resolve_schema_path(project_root)?;
        let snapshot = self.load(&path)?;
        if snapshot.backend != config.backend {
            return Err(
                "database schema snapshot backend does not match `database.backend`".to_string(),
            );
        }
        Ok(snapshot)
    }

    pub fn load(&mut self, path: &Path) -> Result<Arc<DbSchemaSnapshot>, String> {
        let canonical = path
            .canonicalize()
            .map_err(|_| "could not resolve database schema snapshot path".to_string())?;
        let contents = fs::read(&canonical)
            .map_err(|_| "could not read database schema snapshot".to_string())?;
        let fingerprint = fingerprint(&contents);
        if let Some(cached) = self.entries.get(&canonical)
            && cached.fingerprint == fingerprint
        {
            return Ok(Arc::clone(&cached.snapshot));
        }
        let snapshot = DbSchemaSnapshot::load(&canonical)?;
        let snapshot = Arc::new(snapshot);
        if self.entries.len() >= 32 {
            self.entries.clear();
        }
        self.entries.insert(
            canonical,
            CachedDbSchema {
                fingerprint,
                snapshot: Arc::clone(&snapshot),
            },
        );
        Ok(snapshot)
    }
}

fn fingerprint(contents: &[u8]) -> [u8; 32] {
    sha256(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    fn temp_file() -> PathBuf {
        let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("tyrus-db-schema-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root.join("tyrus-db-schema.json")
    }

    fn table(name: &str, column: &str) -> DbTableSchema {
        DbTableSchema {
            engine: None,
            name: name.to_string(),
            columns: vec![DbColumnSchema {
                name: column.to_string(),
                logical_type: "i64".to_string(),
                nullable: false,
                ordinal: 0,
                default: None,
                native_type: None,
            }],
            primary_key: vec![column.to_string()],
            unique: vec![],
            foreign_keys: vec![],
            indexes: vec![],
            triggers: vec![],
        }
    }

    #[test]
    fn snapshots_round_trip_with_a_deterministic_hash() {
        let snapshot =
            DbSchemaSnapshot::new("./data/app.db".to_string(), 7, vec![table("users", "id")]);
        let json = snapshot.to_pretty_json().unwrap();
        let path = temp_file();
        fs::write(&path, json).unwrap();
        let loaded = DbSchemaSnapshot::load(&path).unwrap();
        assert_eq!(loaded, snapshot);
        assert!(loaded.schema_hash.starts_with("sha256:"));
    }

    #[test]
    fn rejects_unknown_and_future_snapshot_versions_explicitly() {
        let path = temp_file();
        fs::write(&path, r#"{"formatVersion":2}"#).unwrap();
        assert!(
            DbSchemaSnapshot::load(&path)
                .unwrap_err()
                .contains("supports 1")
        );
        fs::write(&path, r#"{"formatVersion":1,"backend":"sqlite","source":"./db.sqlite","schemaVersion":1,"schemaHash":"sha256:bad","tables":[],"password":"secret"}"#).unwrap();
        let error = DbSchemaSnapshot::load(&path).unwrap_err();
        assert!(error.contains("invalid shape"));
        assert!(!error.contains("secret"));
    }

    #[test]
    fn rejects_hash_drift_and_invalid_schema_references() {
        let mut snapshot =
            DbSchemaSnapshot::new("./app.db".to_string(), 1, vec![table("users", "id")]);
        snapshot.tables[0].columns[0].logical_type = "string".to_string();
        assert!(snapshot.validate().unwrap_err().contains("schemaHash"));
        snapshot.schema_hash = snapshot.computed_hash();
        snapshot.tables[0].primary_key = vec!["missing".to_string()];
        snapshot.schema_hash = snapshot.computed_hash();
        assert!(snapshot.validate().unwrap_err().contains("unknown column"));
    }

    #[test]
    fn validates_sqlite_identifiers_case_insensitively() {
        let parent = table("Teams", "ID");
        let mut child = table("Memberships", "team_id");
        child.primary_key.clear();
        child.foreign_keys.push(DbForeignKeySchema {
            columns: vec!["TEAM_ID".to_string()],
            referenced_table: "teams".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_update: "NO ACTION".to_string(),
            on_delete: "NO ACTION".to_string(),
        });
        let snapshot = DbSchemaSnapshot::new("./app.db".to_string(), 1, vec![parent, child]);
        assert!(snapshot.validate().is_ok());

        let duplicate_table = DbSchemaSnapshot::new(
            "./app.db".to_string(),
            1,
            vec![table("users", "id"), table("USERS", "other")],
        );
        assert!(
            duplicate_table
                .validate()
                .unwrap_err()
                .contains("duplicate table name")
        );

        let mut duplicate_column = table("users", "id");
        duplicate_column.columns.push(DbColumnSchema {
            name: "ID".to_string(),
            logical_type: "i64".to_string(),
            nullable: false,
            ordinal: 1,
            default: None,
            native_type: None,
        });
        let duplicate_column =
            DbSchemaSnapshot::new("./app.db".to_string(), 1, vec![duplicate_column]);
        assert!(
            duplicate_column
                .validate()
                .unwrap_err()
                .contains("duplicate column name")
        );
    }

    #[test]
    fn rejects_unknown_logical_column_types() {
        let mut snapshot =
            DbSchemaSnapshot::new("./app.db".to_string(), 1, vec![table("users", "id")]);
        snapshot.tables[0].columns[0].logical_type = "jsonb".to_string();
        snapshot.schema_hash = snapshot.computed_hash();
        assert!(
            snapshot
                .validate()
                .unwrap_err()
                .contains("unsupported logical type `jsonb`")
        );
    }

    #[test]
    fn cache_reloads_when_snapshot_contents_change() {
        let path = temp_file();
        let first = DbSchemaSnapshot::new("./app.db".to_string(), 1, vec![table("users", "id")]);
        fs::write(&path, first.to_pretty_json().unwrap()).unwrap();
        let mut cache = DbSchemaCache::default();
        let first_cached = cache.load(&path).unwrap();
        let second = DbSchemaSnapshot::new("./app.db".to_string(), 2, vec![table("users", "id")]);
        fs::write(&path, second.to_pretty_json().unwrap()).unwrap();
        let second_cached = cache.load(&path).unwrap();
        assert_eq!(first_cached.schema_version, 1);
        assert_eq!(second_cached.schema_version, 2);
    }

    #[test]
    fn schema_cache_stays_bounded_across_snapshot_paths() {
        let mut cache = DbSchemaCache::default();
        for version in 0..33 {
            let path = temp_file();
            let snapshot = DbSchemaSnapshot::new(
                format!("./app-{version}.db"),
                version,
                vec![table("users", "id")],
            );
            fs::write(&path, snapshot.to_pretty_json().unwrap()).unwrap();
            cache.load(&path).unwrap();
        }
        assert!(cache.entries.len() <= 32);
    }

    #[test]
    fn configured_cache_loads_the_project_snapshot_without_opening_the_database() {
        let path = temp_file();
        let root = path.parent().unwrap();
        let snapshot = DbSchemaSnapshot::new("./app.db".to_string(), 1, vec![table("users", "id")]);
        fs::write(&path, snapshot.to_pretty_json().unwrap()).unwrap();
        let config = DatabaseConfig {
            backend: DatabaseBackend::Sqlite,
            path: "app.db".to_string(),
            connection_env: None,
            pool: None,
            tls: None,
            sqlite: None,
            schema: "tyrus-db-schema.json".to_string(),
        };
        let mut cache = DbSchemaCache::default();
        let loaded = cache.load_configured(root, &config).unwrap();
        assert_eq!(loaded.schema_hash, snapshot.schema_hash);
        assert_eq!(loaded.tables[0].name, "users");
    }

    #[test]
    fn snapshot_source_cannot_contain_credentials_or_escape_paths() {
        for source in [
            "../secret.db",
            "/etc/passwd",
            "sqlite://user:secret@host/db",
        ] {
            let snapshot = DbSchemaSnapshot::new(source.to_string(), 1, vec![]);
            let error = snapshot.validate().unwrap_err();
            assert!(error.contains("snapshot.source"));
            assert!(!error.contains("secret"));
        }
    }
}
