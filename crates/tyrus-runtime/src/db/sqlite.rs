#![allow(
    dead_code,
    reason = "SQLite runtime backend is introduced before its language API in phase 4"
)]

use super::{DbRow, DbStatement, DbValue};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode as SqlxJournalMode, SqlitePool, SqlitePoolOptions,
    SqliteSynchronous as SqlxSynchronous,
};
use sqlx::{AssertSqlSafe, Column, Row, Sqlite, Transaction, TypeInfo, ValueRef};
use std::str::FromStr;
use std::time::Duration;

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
pub(crate) enum SqliteJournal {
    Wal,
    Delete,
}

#[derive(Clone, Copy)]
pub(crate) enum SqliteSync {
    Full,
    Normal,
}

#[derive(Clone, Copy)]
pub(crate) struct SqliteOptions {
    pub(crate) journal: SqliteJournal,
    pub(crate) synchronous: SqliteSync,
    pub(crate) busy_timeout: Duration,
}

impl Default for SqliteOptions {
    fn default() -> Self {
        Self {
            journal: SqliteJournal::Wal,
            synchronous: SqliteSync::Full,
            busy_timeout: BUSY_TIMEOUT,
        }
    }
}

impl SqliteOptions {
    pub(crate) fn from_json(value: Option<&serde_json::Value>) -> Option<Self> {
        let Some(value) = value else {
            return Some(Self::default());
        };
        let object = value.as_object()?;
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "journalMode" | "synchronous" | "busyTimeoutMs"
            )
        }) {
            return None;
        }
        let journal_mode = object
            .get("journalMode")
            .map(serde_json::Value::as_str)
            .unwrap_or(Some("wal"))?;
        let journal = match journal_mode {
            "wal" => SqliteJournal::Wal,
            "delete" => SqliteJournal::Delete,
            _ => return None,
        };
        let synchronous_name = object
            .get("synchronous")
            .map(serde_json::Value::as_str)
            .unwrap_or(Some("full"))?;
        let synchronous = match synchronous_name {
            "full" => SqliteSync::Full,
            "normal" => SqliteSync::Normal,
            _ => return None,
        };
        let busy_timeout_ms = object
            .get("busyTimeoutMs")
            .map(serde_json::Value::as_u64)
            .unwrap_or(Some(5_000))?;
        if !(1..=120_000).contains(&busy_timeout_ms) {
            return None;
        }
        Some(Self {
            journal,
            synchronous,
            busy_timeout: Duration::from_millis(busy_timeout_ms),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DbRuntimeError {
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
}

#[derive(Debug)]
pub(crate) struct DbExecResult {
    pub(crate) rows_affected: u64,
    pub(crate) last_insert_rowid: i64,
}

#[derive(Clone)]
pub(crate) struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub(crate) async fn open(path: &str) -> Result<Self, DbRuntimeError> {
        Self::open_with_options(path, SqliteOptions::default()).await
    }

    #[cfg(test)]
    async fn open_with_busy_timeout(
        path: &str,
        busy_timeout: Duration,
    ) -> Result<Self, DbRuntimeError> {
        Self::open_with_options(
            path,
            SqliteOptions {
                busy_timeout,
                ..SqliteOptions::default()
            },
        )
        .await
    }

    pub(crate) async fn open_with_options(
        path: &str,
        settings: SqliteOptions,
    ) -> Result<Self, DbRuntimeError> {
        let in_memory = path == ":memory:";
        let options = if in_memory {
            SqliteConnectOptions::from_str("sqlite::memory:")
                .map_err(|_| DbRuntimeError::connection())?
        } else {
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
        }
        .foreign_keys(true)
        .busy_timeout(settings.busy_timeout)
        .synchronous(match settings.synchronous {
            SqliteSync::Full => SqlxSynchronous::Full,
            SqliteSync::Normal => SqlxSynchronous::Normal,
        })
        .journal_mode(if in_memory {
            SqlxJournalMode::Memory
        } else {
            match settings.journal {
                SqliteJournal::Wal => SqlxJournalMode::Wal,
                SqliteJournal::Delete => SqlxJournalMode::Delete,
            }
        })
        .statement_cache_capacity(100);

        let pool = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(|error| map_sqlite_error(&error, SqliteOperation::Connect))?;
        if let Err(error) = verify_sqlite_options(&pool, in_memory, settings).await {
            pool.close().await;
            return Err(error);
        }

        Ok(Self { pool })
    }

    pub(crate) async fn close(self) {
        self.pool.close().await;
    }

    pub(crate) async fn execute(
        &self,
        statement: &DbStatement,
    ) -> Result<DbExecResult, DbRuntimeError> {
        let (sql, params) = statement.parts();
        let result = bind_params(sqlx::query(AssertSqlSafe(sql.to_owned())), params)?
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlite_error(&error, SqliteOperation::Execute))?;
        Ok(DbExecResult {
            rows_affected: result.rows_affected(),
            last_insert_rowid: result.last_insert_rowid(),
        })
    }

    pub(crate) async fn query_many(
        &self,
        statement: &DbStatement,
    ) -> Result<Vec<DbRow>, DbRuntimeError> {
        let (sql, params) = statement.parts();
        let rows = bind_params(sqlx::query(AssertSqlSafe(sql.to_owned())), params)?
            .fetch_all(&self.pool)
            .await
            .map_err(|error| map_sqlite_error(&error, SqliteOperation::Query))?;
        rows.iter().map(decode_row).collect::<Result<Vec<_>, _>>()
    }

    pub(crate) async fn query_one(
        &self,
        statement: &DbStatement,
    ) -> Result<Option<DbRow>, DbRuntimeError> {
        let (sql, params) = statement.parts();
        let row = bind_params(sqlx::query(AssertSqlSafe(sql.to_owned())), params)?
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| map_sqlite_error(&error, SqliteOperation::Query))?;
        row.as_ref().map(decode_row).transpose()
    }

    pub(crate) async fn begin(&self) -> Result<Transaction<'static, Sqlite>, DbRuntimeError> {
        self.pool
            .begin()
            .await
            .map_err(|error| map_sqlite_error(&error, SqliteOperation::Transaction))
    }

    pub(crate) async fn execute_in(
        &self,
        transaction: &mut Transaction<'static, Sqlite>,
        statement: &DbStatement,
    ) -> Result<DbExecResult, DbRuntimeError> {
        let (sql, params) = statement.parts();
        let result = bind_params(sqlx::query(AssertSqlSafe(sql.to_owned())), params)?
            .execute(&mut **transaction)
            .await
            .map_err(|error| map_sqlite_error(&error, SqliteOperation::Execute))?;
        Ok(DbExecResult {
            rows_affected: result.rows_affected(),
            last_insert_rowid: result.last_insert_rowid(),
        })
    }

    pub(crate) async fn query_many_in(
        &self,
        transaction: &mut Transaction<'static, Sqlite>,
        statement: &DbStatement,
    ) -> Result<Vec<DbRow>, DbRuntimeError> {
        let (sql, params) = statement.parts();
        let rows = bind_params(sqlx::query(AssertSqlSafe(sql.to_owned())), params)?
            .fetch_all(&mut **transaction)
            .await
            .map_err(|error| map_sqlite_error(&error, SqliteOperation::Query))?;
        rows.iter().map(decode_row).collect::<Result<Vec<_>, _>>()
    }

    pub(crate) async fn query_one_in(
        &self,
        transaction: &mut Transaction<'static, Sqlite>,
        statement: &DbStatement,
    ) -> Result<Option<DbRow>, DbRuntimeError> {
        let (sql, params) = statement.parts();
        let row = bind_params(sqlx::query(AssertSqlSafe(sql.to_owned())), params)?
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|error| map_sqlite_error(&error, SqliteOperation::Query))?;
        row.as_ref().map(decode_row).transpose()
    }

    pub(crate) async fn commit(
        transaction: Transaction<'static, Sqlite>,
    ) -> Result<(), DbRuntimeError> {
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlite_error(&error, SqliteOperation::Transaction))
    }

    pub(crate) async fn rollback(
        transaction: Transaction<'static, Sqlite>,
    ) -> Result<(), DbRuntimeError> {
        transaction
            .rollback()
            .await
            .map_err(|error| map_sqlite_error(&error, SqliteOperation::Transaction))
    }

    pub(crate) async fn execute_transaction(
        &self,
        statements: &[DbStatement],
    ) -> Result<Vec<DbExecResult>, DbRuntimeError> {
        let mut transaction = self.begin().await?;
        let result = async {
            let mut results = Vec::with_capacity(statements.len());
            for statement in statements {
                results.push(self.execute_in(&mut transaction, statement).await?);
            }
            Ok(results)
        }
        .await;
        match result {
            Ok(results) => {
                Self::commit(transaction).await?;
                Ok(results)
            }
            Err(error) => {
                let _ = Self::rollback(transaction).await;
                Err(error)
            }
        }
    }
}

async fn verify_sqlite_options(
    pool: &SqlitePool,
    in_memory: bool,
    settings: SqliteOptions,
) -> Result<(), DbRuntimeError> {
    let journal_mode = sqlx::query_scalar::<Sqlite, String>("PRAGMA journal_mode")
        .fetch_one(pool)
        .await
        .map_err(|_| sqlite_configuration_error())?;
    let expected_journal_mode = if in_memory {
        "memory"
    } else {
        match settings.journal {
            SqliteJournal::Wal => "wal",
            SqliteJournal::Delete => "delete",
        }
    };
    let synchronous = sqlx::query_scalar::<Sqlite, i64>("PRAGMA synchronous")
        .fetch_one(pool)
        .await
        .map_err(|_| sqlite_configuration_error())?;
    let expected_synchronous = match settings.synchronous {
        SqliteSync::Full => 2,
        SqliteSync::Normal => 1,
    };
    let foreign_keys = sqlx::query_scalar::<Sqlite, i64>("PRAGMA foreign_keys")
        .fetch_one(pool)
        .await
        .map_err(|_| sqlite_configuration_error())?;
    let busy_timeout = sqlx::query_scalar::<Sqlite, i64>("PRAGMA busy_timeout")
        .fetch_one(pool)
        .await
        .map_err(|_| sqlite_configuration_error())?;
    if !journal_mode.eq_ignore_ascii_case(expected_journal_mode)
        || synchronous != expected_synchronous
        || foreign_keys != 1
        || u64::try_from(busy_timeout).ok() != Some(settings.busy_timeout.as_millis() as u64)
    {
        return Err(sqlite_configuration_error());
    }
    Ok(())
}

fn sqlite_configuration_error() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_INVALID_CONFIGURATION",
        message: "SQLite did not apply the configured journal, synchronization, or locking settings",
    }
}

fn bind_params(
    mut query: sqlx::query::Query<'static, Sqlite, sqlx::sqlite::SqliteArguments>,
    values: &[DbValue],
) -> Result<sqlx::query::Query<'static, Sqlite, sqlx::sqlite::SqliteArguments>, DbRuntimeError> {
    for value in values {
        query = match value {
            DbValue::Null => query.bind(Option::<String>::None),
            DbValue::Bool(value) => query.bind(i64::from(*value)),
            DbValue::Signed(value) => query.bind(*value),
            DbValue::Unsigned(value) => {
                query.bind(i64::try_from(*value).map_err(|_| DbRuntimeError {
                    code: "ERR_DB_VALUE_OUT_OF_RANGE",
                    message: "Unsigned integer is outside the SQLite integer range",
                })?)
            }
            DbValue::Real(value) => query.bind(*value),
            DbValue::Text(value) => query.bind(value.clone()),
            DbValue::Blob(value) => query.bind(value.clone()),
            DbValue::Instant {
                seconds,
                nanoseconds,
            } => {
                let timestamp = jiff::Timestamp::new(
                    *seconds,
                    i32::try_from(*nanoseconds).map_err(|_| DbRuntimeError::decode())?,
                )
                .map_err(|_| DbRuntimeError::decode())?;
                query.bind(timestamp.to_string())
            }
            DbValue::Json(_) => {
                return Err(DbRuntimeError {
                    code: "ERR_DB_UNSUPPORTED_VALUE",
                    message: "structured JSON parameters are available only for MongoDB Models",
                });
            }
        };
    }
    Ok(query)
}

fn checked_sqlite_integer(value: u64) -> Option<i64> {
    i64::try_from(value).ok()
}

fn decode_row(row: &sqlx::sqlite::SqliteRow) -> Result<DbRow, DbRuntimeError> {
    let mut columns = Vec::with_capacity(row.len());
    let mut values = Vec::with_capacity(row.len());
    for index in 0..row.len() {
        columns.push(row.column(index).name().to_string());
        let raw = row
            .try_get_raw(index)
            .map_err(|_| DbRuntimeError::decode())?;
        let value = if raw.is_null() {
            DbValue::Null
        } else {
            match raw.type_info().name() {
                "INTEGER" => DbValue::Signed(
                    row.try_get::<i64, _>(index)
                        .map_err(|_| DbRuntimeError::decode())?,
                ),
                "REAL" => DbValue::Real(
                    row.try_get::<f64, _>(index)
                        .map_err(|_| DbRuntimeError::decode())?,
                ),
                "TEXT" => DbValue::Text(
                    row.try_get::<String, _>(index)
                        .map_err(|_| DbRuntimeError::decode())?,
                ),
                "BLOB" => DbValue::Blob(
                    row.try_get::<Vec<u8>, _>(index)
                        .map_err(|_| DbRuntimeError::decode())?,
                ),
                _ => return Err(DbRuntimeError::decode()),
            }
        };
        values.push(value);
    }
    Ok(DbRow::with_columns(columns, values))
}

#[derive(Clone, Copy)]
enum SqliteOperation {
    Connect,
    Execute,
    Query,
    Transaction,
}

fn map_sqlite_error(error: &sqlx::Error, operation: SqliteOperation) -> DbRuntimeError {
    if let sqlx::Error::Database(database_error) = error {
        if matches!(
            database_error.kind(),
            sqlx::error::ErrorKind::UniqueViolation
                | sqlx::error::ErrorKind::ForeignKeyViolation
                | sqlx::error::ErrorKind::NotNullViolation
                | sqlx::error::ErrorKind::CheckViolation
        ) {
            return DbRuntimeError {
                code: "ERR_DB_SQLITE_CONSTRAINT",
                message: "SQLite constraint failed",
            };
        }
        let is_busy = database_error
            .code()
            .and_then(|code| code.parse::<u32>().ok())
            .is_some_and(|code| matches!(code & 0xff, 5 | 6));
        if is_busy {
            return DbRuntimeError {
                code: "ERR_DB_SQLITE_BUSY",
                message: "SQLite database is busy",
            };
        }
    }

    match operation {
        SqliteOperation::Connect => DbRuntimeError::connection(),
        SqliteOperation::Execute => DbRuntimeError {
            code: "ERR_DB_EXECUTE_FAILED",
            message: "SQLite execute failed",
        },
        SqliteOperation::Query => DbRuntimeError {
            code: "ERR_DB_QUERY_FAILED",
            message: "SQLite query failed",
        },
        SqliteOperation::Transaction => DbRuntimeError {
            code: "ERR_DB_TRANSACTION_FAILED",
            message: "SQLite transaction failed",
        },
    }
}

impl DbRuntimeError {
    fn connection() -> Self {
        Self {
            code: "ERR_DB_CONNECTION_FAILED",
            message: "SQLite connection failed",
        }
    }

    pub(super) fn decode() -> Self {
        Self {
            code: "ERR_DB_DECODE_FAILED",
            message: "database returned an unsupported value",
        }
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::checked_sqlite_integer;

    #[kani::proof]
    fn sqlite_unsigned_integer_conversion_is_exact_or_rejected() {
        let value: u64 = kani::any();
        match checked_sqlite_integer(value) {
            Some(converted) => {
                assert!(converted >= 0);
                assert!(converted as u64 == value);
            }
            None => assert!(value > i64::MAX as u64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);

    fn temp_database_path() -> String {
        let id = NEXT_DATABASE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!("tyrus-db-{}-{id}.sqlite", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    fn statement(sql: &str, params: Vec<DbValue>) -> DbStatement {
        DbStatement::new(sql.into(), params)
    }

    #[tokio::test]
    async fn sqlite_opens_with_explicit_defaults_and_executes_parameterized_values() {
        let db = SqliteDatabase::open(":memory:").await.unwrap();
        db.execute(&statement(
            "CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT NOT NULL, enabled INTEGER, serial INTEGER, payload BLOB, note TEXT)",
            vec![],
        ))
        .await
        .unwrap();
        let inserted = db
            .execute(&statement(
                "INSERT INTO sample (name, enabled, serial, payload, note) VALUES (?, ?, ?, ?, ?)",
                vec![
                    DbValue::Text("Tyrus".to_string()),
                    DbValue::Bool(true),
                    DbValue::Unsigned(23),
                    DbValue::Blob(vec![0, 1, 255]),
                    DbValue::Null,
                ],
            ))
            .await
            .unwrap();
        assert_eq!(inserted.rows_affected, 1);
        assert_eq!(inserted.last_insert_rowid, 1);

        let rows = db
            .query_many(&statement(
                "SELECT id, name, enabled, serial, payload, note FROM sample WHERE id = ?",
                vec![DbValue::Signed(1)],
            ))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].values(),
            &[
                DbValue::Signed(1),
                DbValue::Text("Tyrus".to_string()),
                DbValue::Signed(1),
                DbValue::Signed(23),
                DbValue::Blob(vec![0, 1, 255]),
                DbValue::Null,
            ]
        );
        for (pragma, expected) in [
            ("PRAGMA foreign_keys", DbValue::Signed(1)),
            ("PRAGMA busy_timeout", DbValue::Signed(5000)),
            ("PRAGMA journal_mode", DbValue::Text("memory".to_string())),
            ("PRAGMA synchronous", DbValue::Signed(2)),
        ] {
            let rows = db.query_many(&statement(pragma, vec![])).await.unwrap();
            assert_eq!(rows[0].values(), &[expected]);
        }
        db.close().await;
    }

    #[tokio::test]
    async fn sqlite_query_one_returns_none_for_no_match_and_first_row_for_match() {
        let db = SqliteDatabase::open(":memory:").await.unwrap();
        db.execute(&statement("CREATE TABLE sample (value REAL)", vec![]))
            .await
            .unwrap();
        assert!(
            db.query_one(&statement("SELECT value FROM sample", vec![]))
                .await
                .unwrap()
                .is_none()
        );
        db.execute(&statement(
            "INSERT INTO sample VALUES (?)",
            vec![DbValue::Real(2.5)],
        ))
        .await
        .unwrap();
        assert_eq!(
            db.query_one(&statement("SELECT value FROM sample", vec![]))
                .await
                .unwrap()
                .unwrap()
                .values(),
            &[DbValue::Real(2.5)]
        );
        db.close().await;
    }

    #[tokio::test]
    async fn sqlite_constraints_return_a_stable_recoverable_error() {
        let db = SqliteDatabase::open(":memory:").await.unwrap();
        db.execute(&statement(
            "CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT UNIQUE)",
            vec![],
        ))
        .await
        .unwrap();
        db.execute(&statement(
            "INSERT INTO sample (name) VALUES (?)",
            vec![DbValue::Text("same".to_string())],
        ))
        .await
        .unwrap();
        let error = db
            .execute(&statement(
                "INSERT INTO sample (name) VALUES (?)",
                vec![DbValue::Text("same".to_string())],
            ))
            .await
            .unwrap_err();
        assert_eq!(error.code, "ERR_DB_SQLITE_CONSTRAINT");
        assert_eq!(error.message, "SQLite constraint failed");
        db.close().await;
    }

    #[tokio::test]
    async fn sqlite_transaction_commits_and_rolls_back_explicitly() {
        let db = SqliteDatabase::open(":memory:").await.unwrap();
        db.execute(&statement("CREATE TABLE sample (value INTEGER)", vec![]))
            .await
            .unwrap();
        let mut transaction = db.begin().await.unwrap();
        db.execute_in(
            &mut transaction,
            &statement("INSERT INTO sample VALUES (?)", vec![DbValue::Signed(1)]),
        )
        .await
        .unwrap();
        SqliteDatabase::commit(transaction).await.unwrap();

        let transaction = db.begin().await.unwrap();
        let mut transaction = transaction;
        db.execute_in(
            &mut transaction,
            &statement("INSERT INTO sample VALUES (?)", vec![DbValue::Signed(2)]),
        )
        .await
        .unwrap();
        SqliteDatabase::rollback(transaction).await.unwrap();

        let rows = db
            .query_many(&statement("SELECT value FROM sample", vec![]))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].values(), &[DbValue::Signed(1)]);
        db.close().await;
    }

    #[tokio::test]
    async fn sqlite_transaction_batch_commits_all_statements_or_rolls_back_all() {
        let db = SqliteDatabase::open(":memory:").await.unwrap();
        db.execute(&statement(
            "CREATE TABLE sample (value INTEGER UNIQUE)",
            vec![],
        ))
        .await
        .unwrap();

        let committed = db
            .execute_transaction(&[
                statement("INSERT INTO sample VALUES (?)", vec![DbValue::Signed(1)]),
                statement("INSERT INTO sample VALUES (?)", vec![DbValue::Signed(2)]),
            ])
            .await
            .unwrap();
        assert_eq!(committed.len(), 2);
        assert_eq!(committed[0].rows_affected, 1);
        assert_eq!(committed[1].rows_affected, 1);

        let failed = db
            .execute_transaction(&[
                statement("INSERT INTO sample VALUES (?)", vec![DbValue::Signed(3)]),
                statement("INSERT INTO sample VALUES (?)", vec![DbValue::Signed(1)]),
            ])
            .await;
        assert_eq!(failed.unwrap_err().code, "ERR_DB_SQLITE_CONSTRAINT");

        let rows = db
            .query_many(&statement(
                "SELECT value FROM sample ORDER BY value",
                vec![],
            ))
            .await
            .unwrap();
        assert_eq!(
            rows.iter()
                .map(|row| match row.values()[0] {
                    DbValue::Signed(value) => value,
                    _ => panic!("expected a signed SQLite integer"),
                })
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        db.close().await;
    }

    #[tokio::test]
    async fn sqlite_file_connections_enable_wal() {
        let path = temp_database_path();
        let db = SqliteDatabase::open(&path).await.unwrap();
        let rows = db
            .query_many(&statement("PRAGMA journal_mode", vec![]))
            .await
            .unwrap();
        assert_eq!(rows[0].values(), &[DbValue::Text("wal".to_string())]);
        db.close().await;
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn sqlite_wal_checkpoint_backup_and_restart_preserve_committed_rows() {
        let path = temp_database_path();
        let backup_path = format!("{path}.backup");
        let settings = SqliteOptions {
            journal: SqliteJournal::Wal,
            synchronous: SqliteSync::Full,
            ..SqliteOptions::default()
        };
        let db = SqliteDatabase::open_with_options(&path, settings)
            .await
            .unwrap();
        db.execute(&statement(
            "CREATE TABLE durable_rows (id INTEGER PRIMARY KEY, value TEXT NOT NULL)",
            vec![],
        ))
        .await
        .unwrap();
        db.execute(&statement(
            "INSERT INTO durable_rows VALUES (?, ?)",
            vec![DbValue::Signed(1), DbValue::Text("committed".to_string())],
        ))
        .await
        .unwrap();

        let checkpoint = db
            .query_one(&statement("PRAGMA wal_checkpoint(TRUNCATE)", vec![]))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(checkpoint.values()[0], DbValue::Signed(0));

        db.execute(&statement(
            "VACUUM INTO ?",
            vec![DbValue::Text(backup_path.clone())],
        ))
        .await
        .unwrap();
        db.close().await;

        let reopened = SqliteDatabase::open_with_options(&path, settings)
            .await
            .unwrap();
        let journal = reopened
            .query_one(&statement("PRAGMA journal_mode", vec![]))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(journal.values(), &[DbValue::Text("wal".to_string())]);
        let rows = reopened
            .query_many(&statement("SELECT id, value FROM durable_rows", vec![]))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].values()[0], DbValue::Signed(1));
        assert_eq!(rows[0].values()[1], DbValue::Text("committed".to_string()));
        reopened.close().await;

        let backup = SqliteDatabase::open(&backup_path).await.unwrap();
        let backup_rows = backup
            .query_many(&statement("SELECT id, value FROM durable_rows", vec![]))
            .await
            .unwrap();
        assert_eq!(backup_rows.len(), 1);
        assert_eq!(backup_rows[0].values(), rows[0].values());
        backup.close().await;

        for file in [path.clone(), backup_path.clone()] {
            let _ = std::fs::remove_file(&file);
            let _ = std::fs::remove_file(format!("{file}-wal"));
            let _ = std::fs::remove_file(format!("{file}-shm"));
        }
    }

    #[tokio::test]
    async fn sqlite_generated_rowids_are_non_negative_i64_values() {
        let db = SqliteDatabase::open(":memory:").await.unwrap();
        db.execute(&statement(
            "CREATE TABLE generated_ids (id INTEGER PRIMARY KEY, value TEXT NOT NULL)",
            vec![],
        ))
        .await
        .unwrap();
        let mut previous = 0;
        for value in ["first", "second", "third"] {
            let result = db
                .execute(&statement(
                    "INSERT INTO generated_ids (value) VALUES (?)",
                    vec![DbValue::Text(value.to_string())],
                ))
                .await
                .unwrap();
            assert!(result.last_insert_rowid > previous);
            assert!(result.last_insert_rowid >= 0);
            previous = result.last_insert_rowid;
        }
        db.close().await;
    }

    #[tokio::test]
    async fn sqlite_journal_and_sync_modes_survive_connection_restarts() {
        for (journal, synchronous, expected_journal, expected_sync) in [
            (SqliteJournal::Wal, SqliteSync::Full, "wal", 2),
            (SqliteJournal::Wal, SqliteSync::Normal, "wal", 1),
            (SqliteJournal::Delete, SqliteSync::Full, "delete", 2),
            (SqliteJournal::Delete, SqliteSync::Normal, "delete", 1),
        ] {
            let path = temp_database_path();
            let settings = SqliteOptions {
                journal,
                synchronous,
                ..SqliteOptions::default()
            };
            let db = SqliteDatabase::open_with_options(&path, settings)
                .await
                .unwrap();
            for (pragma, expected) in [
                (
                    "PRAGMA journal_mode",
                    DbValue::Text(expected_journal.to_string()),
                ),
                ("PRAGMA synchronous", DbValue::Signed(expected_sync)),
            ] {
                let row = db
                    .query_one(&statement(pragma, vec![]))
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(row.values(), &[expected]);
            }
            db.execute(&statement("CREATE TABLE mode_rows (value INTEGER)", vec![]))
                .await
                .unwrap();
            db.execute(&statement("INSERT INTO mode_rows VALUES (7)", vec![]))
                .await
                .unwrap();
            db.close().await;

            let reopened = SqliteDatabase::open_with_options(&path, settings)
                .await
                .unwrap();
            let row = reopened
                .query_one(&statement("SELECT value FROM mode_rows", vec![]))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(row.values(), &[DbValue::Signed(7)]);
            reopened.close().await;
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(format!("{path}-wal"));
            let _ = std::fs::remove_file(format!("{path}-shm"));
        }
    }

    #[tokio::test]
    async fn sqlite_instant_bindings_preserve_day_boundaries_and_nanoseconds() {
        let db = SqliteDatabase::open(":memory:").await.unwrap();
        for (seconds, nanoseconds, expected) in [
            (1_735_689_599, 999_999_999, "2024-12-31T23:59:59.999999999Z"),
            (1_735_689_600, 0, "2025-01-01T00:00:00Z"),
            (-1, 999_999_999, "1969-12-31T23:59:59.999999999Z"),
        ] {
            let row = db
                .query_one(&statement(
                    "SELECT ? AS instant",
                    vec![DbValue::Instant {
                        seconds,
                        nanoseconds,
                    }],
                ))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(row.values(), &[DbValue::Text(expected.to_string())]);
        }
        db.close().await;
    }

    #[tokio::test]
    async fn sqlite_wal_concurrent_process_writers_are_visible() {
        const PATH_ENV: &str = "TYRUS_INTERNAL_DB_WAL_TEST_PATH";
        const WRITER_ENV: &str = "TYRUS_INTERNAL_DB_WAL_TEST_WRITER";
        if std::env::var(WRITER_ENV).as_deref() == Ok("1") {
            let path = std::env::var(PATH_ENV).unwrap();
            let writer = std::env::var("TYRUS_INTERNAL_DB_WAL_TEST_ID").unwrap();
            let db = loop {
                match SqliteDatabase::open_with_options(&path, SqliteOptions::default()).await {
                    Ok(db) => break db,
                    Err(error) if error.code == "ERR_DB_SQLITE_BUSY" => {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Err(error) => panic!("writer could not open database: {error:?}"),
                }
            };
            for sequence in 0..20 {
                let insert = statement(
                    "INSERT INTO process_rows (writer, sequence) VALUES (?, ?)",
                    vec![DbValue::Text(writer.clone()), DbValue::Signed(sequence)],
                );
                let mut attempts = 0;
                loop {
                    match db.execute(&insert).await {
                        Ok(_) => break,
                        Err(error) if error.code == "ERR_DB_SQLITE_BUSY" && attempts < 100 => {
                            attempts += 1;
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        Err(error) => panic!("writer could not insert row: {error:?}"),
                    }
                }
            }
            db.close().await;
            return;
        }

        let path = temp_database_path();
        let db = SqliteDatabase::open_with_options(&path, SqliteOptions::default())
            .await
            .unwrap();
        db.execute(&statement(
            "CREATE TABLE process_rows (writer TEXT NOT NULL, sequence INTEGER NOT NULL)",
            vec![],
        ))
        .await
        .unwrap();
        db.close().await;

        let test_binary = std::env::current_exe().unwrap();
        let spawn_writer = |id: &str| {
            Command::new(&test_binary)
                .arg("--exact")
                .arg("db::sqlite::tests::sqlite_wal_concurrent_process_writers_are_visible")
                .arg("--nocapture")
                .env(PATH_ENV, &path)
                .env(WRITER_ENV, "1")
                .env("TYRUS_INTERNAL_DB_WAL_TEST_ID", id)
                .spawn()
                .unwrap()
        };
        let first = spawn_writer("first");
        let second = spawn_writer("second");
        let first_output = first.wait_with_output().unwrap();
        let second_output = second.wait_with_output().unwrap();
        assert!(
            first_output.status.success(),
            "first writer failed: {}",
            String::from_utf8_lossy(&first_output.stderr)
        );
        assert!(
            second_output.status.success(),
            "second writer failed: {}",
            String::from_utf8_lossy(&second_output.stderr)
        );

        let db = SqliteDatabase::open_with_options(&path, SqliteOptions::default())
            .await
            .unwrap();
        let rows = db
            .query_many(&statement(
                "SELECT writer, sequence FROM process_rows",
                vec![],
            ))
            .await
            .unwrap();
        assert_eq!(rows.len(), 40);
        assert_eq!(
            rows.iter()
                .filter(|row| row.values()[0] == DbValue::Text("first".to_string()))
                .count(),
            20
        );
        assert_eq!(
            rows.iter()
                .filter(|row| row.values()[0] == DbValue::Text("second".to_string()))
                .count(),
            20
        );
        db.close().await;
        for file in [path.clone(), format!("{path}-wal"), format!("{path}-shm")] {
            let _ = std::fs::remove_file(file);
        }
    }

    #[tokio::test]
    async fn sqlite_busy_timeout_maps_to_a_stable_error_code() {
        let path = temp_database_path();
        let first = SqliteDatabase::open_with_busy_timeout(&path, Duration::from_millis(25))
            .await
            .unwrap();
        let second = SqliteDatabase::open_with_busy_timeout(&path, Duration::from_millis(25))
            .await
            .unwrap();
        first
            .execute(&statement("CREATE TABLE sample (value INTEGER)", vec![]))
            .await
            .unwrap();
        let transaction = first.begin().await.unwrap();
        let mut transaction = transaction;
        first
            .execute_in(
                &mut transaction,
                &statement("INSERT INTO sample VALUES (1)", vec![]),
            )
            .await
            .unwrap();
        let error = second
            .execute(&statement("INSERT INTO sample VALUES (2)", vec![]))
            .await
            .unwrap_err();
        assert_eq!(error.code, "ERR_DB_SQLITE_BUSY");
        SqliteDatabase::rollback(transaction).await.unwrap();
        first.close().await;
        second.close().await;
        let _ = std::fs::remove_file(path);
    }
    #[tokio::test]
    async fn sqlite_rejects_unsigned_values_outside_its_integer_range() {
        let db = SqliteDatabase::open(":memory:").await.unwrap();
        let error = db
            .execute(&statement("SELECT ?", vec![DbValue::Unsigned(u64::MAX)]))
            .await
            .unwrap_err();
        assert_eq!(error.code, "ERR_DB_VALUE_OUT_OF_RANGE");
        db.close().await;
    }

    #[tokio::test]
    async fn sqlite_open_errors_are_stable_and_redact_the_database_path() {
        let path = format!("{}/missing/database.sqlite", temp_database_path());
        let error = SqliteDatabase::open(&path).await.err().unwrap();
        assert_eq!(error.code, "ERR_DB_CONNECTION_FAILED");
        assert!(!error.message.contains(&path));
    }

    #[tokio::test]
    async fn cancelling_a_busy_write_does_not_poison_the_connection_pool() {
        let path = temp_database_path();
        let first = SqliteDatabase::open_with_busy_timeout(&path, Duration::from_secs(1))
            .await
            .unwrap();
        let second = SqliteDatabase::open_with_busy_timeout(&path, Duration::from_secs(1))
            .await
            .unwrap();
        first
            .execute(&statement("CREATE TABLE sample (value INTEGER)", vec![]))
            .await
            .unwrap();
        let transaction = first.begin().await.unwrap();
        let mut transaction = transaction;
        first
            .execute_in(
                &mut transaction,
                &statement("INSERT INTO sample VALUES (1)", vec![]),
            )
            .await
            .unwrap();

        let pending = tokio::spawn(async move {
            second
                .execute(&statement("INSERT INTO sample VALUES (2)", vec![]))
                .await
        });
        tokio::time::sleep(Duration::from_millis(25)).await;
        pending.abort();
        assert!(pending.await.unwrap_err().is_cancelled());
        SqliteDatabase::rollback(transaction).await.unwrap();

        first.close().await;
        let result = SqliteDatabase::open_with_busy_timeout(&path, Duration::from_secs(1))
            .await
            .unwrap();
        result
            .execute(&statement("INSERT INTO sample VALUES (3)", vec![]))
            .await
            .unwrap();
        result.close().await;
        let _ = std::fs::remove_file(path);
    }
}
