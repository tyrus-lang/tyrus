use super::sqlite::{DbExecResult, DbRuntimeError};
use super::{DbRow, DbStatement, DbValue};
use sqlx::any::{AnyPoolOptions, AnyTypeInfoKind};
use sqlx::{Any, AssertSqlSafe, Column, Connection, Row, Transaction, ValueRef};
use std::time::Duration;

const UUID_PARAMETER_MARKER: &str = "?/*__tyrus_uuid_parameter__*/";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ServerBackend {
    Postgresql,
    Mysql,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ServerPoolOptions {
    max_connections: u32,
    connect_timeout_ms: u64,
    acquire_timeout_ms: u64,
    query_timeout_ms: u64,
    idle_timeout_ms: u64,
    max_lifetime_ms: u64,
}

impl Default for ServerPoolOptions {
    fn default() -> Self {
        Self {
            max_connections: 10,
            connect_timeout_ms: 10_000,
            acquire_timeout_ms: 5_000,
            query_timeout_ms: 30_000,
            idle_timeout_ms: 30_000,
            max_lifetime_ms: 1_800_000,
        }
    }
}

impl ServerPoolOptions {
    pub(crate) fn from_json(value: &serde_json::Value) -> Option<Self> {
        let object = value.as_object()?;
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "maxConnections"
                    | "connectTimeoutMs"
                    | "acquireTimeoutMs"
                    | "queryTimeoutMs"
                    | "idleTimeoutMs"
                    | "maxLifetimeMs"
            )
        }) {
            return None;
        }
        let mut options = Self::default();
        if let Some(value) = json_u64(object.get("maxConnections"))? {
            options.max_connections = u32::try_from(value).ok()?;
        }
        options.connect_timeout_ms =
            json_u64(object.get("connectTimeoutMs"))?.unwrap_or(options.connect_timeout_ms);
        options.acquire_timeout_ms =
            json_u64(object.get("acquireTimeoutMs"))?.unwrap_or(options.acquire_timeout_ms);
        options.query_timeout_ms =
            json_u64(object.get("queryTimeoutMs"))?.unwrap_or(options.query_timeout_ms);
        options.idle_timeout_ms =
            json_u64(object.get("idleTimeoutMs"))?.unwrap_or(options.idle_timeout_ms);
        options.max_lifetime_ms =
            json_u64(object.get("maxLifetimeMs"))?.unwrap_or(options.max_lifetime_ms);
        Some(options)
    }

    pub(crate) fn validate(self) -> bool {
        (1..=100).contains(&self.max_connections)
            && (1..=120_000).contains(&self.connect_timeout_ms)
            && (1..=120_000).contains(&self.acquire_timeout_ms)
            && (1..=120_000).contains(&self.query_timeout_ms)
            && (1..=86_400_000).contains(&self.idle_timeout_ms)
            && (1..=86_400_000).contains(&self.max_lifetime_ms)
    }
}

fn json_u64(value: Option<&serde_json::Value>) -> Option<Option<u64>> {
    value.map_or(Some(None), |value| value.as_u64().map(Some))
}

#[derive(Clone)]
pub(crate) struct ServerDatabase {
    pool: sqlx::Pool<Any>,
    backend: ServerBackend,
    query_timeout_ms: u64,
}

impl ServerDatabase {
    pub(crate) async fn close(self) {
        self.pool.close().await;
    }

    pub(crate) async fn begin_transaction(
        &self,
    ) -> Result<Transaction<'static, Any>, DbRuntimeError> {
        self.pool
            .begin()
            .await
            .map_err(|error| backend_operation_error(error, transaction_error()))
    }

    pub(crate) async fn execute_in_transaction(
        &self,
        transaction: &mut Transaction<'static, Any>,
        statement: &DbStatement,
    ) -> Result<DbExecResult, DbRuntimeError> {
        tokio::time::timeout(Duration::from_millis(self.query_timeout_ms), async {
            let (sql, params) = statement.parts();
            let sql = prepare_sql(sql, self.backend, params)?;
            if self.backend == ServerBackend::Postgresql
                && sql.contains("AS \"__tyrus_generated_id\"")
            {
                let row =
                    bind_params(sqlx::query::<Any>(AssertSqlSafe(sql)), params, self.backend)?
                        .fetch_optional(&mut **transaction)
                        .await
                        .map_err(|error| backend_operation_error(error, execute_error()))?
                        .ok_or_else(execute_error)?;
                let generated_id = row
                    .try_get::<i64, _>("__tyrus_generated_id")
                    .map_err(|_| DbRuntimeError::decode())?;
                return Ok(DbExecResult {
                    rows_affected: 1,
                    last_insert_rowid: generated_id,
                });
            }
            let result = bind_params(sqlx::query::<Any>(AssertSqlSafe(sql)), params, self.backend)?
                .execute(&mut **transaction)
                .await
                .map_err(|error| backend_operation_error(error, execute_error()))?;
            Ok(DbExecResult {
                rows_affected: result.rows_affected(),
                last_insert_rowid: result.last_insert_id().unwrap_or(0),
            })
        })
        .await
        .map_err(|_| query_timeout_error())?
    }

    pub(crate) async fn query_many_in_transaction(
        &self,
        transaction: &mut Transaction<'static, Any>,
        statement: &DbStatement,
    ) -> Result<Vec<DbRow>, DbRuntimeError> {
        tokio::time::timeout(Duration::from_millis(self.query_timeout_ms), async {
            let (sql, params) = statement.parts();
            let sql = prepare_sql(sql, self.backend, params)?;
            let rows = bind_params(sqlx::query::<Any>(AssertSqlSafe(sql)), params, self.backend)?
                .fetch_all(&mut **transaction)
                .await
                .map_err(|error| backend_operation_error(error, query_error()))?;
            rows.iter().map(decode_row).collect()
        })
        .await
        .map_err(|_| query_timeout_error())?
    }

    pub(crate) async fn query_one_in_transaction(
        &self,
        transaction: &mut Transaction<'static, Any>,
        statement: &DbStatement,
    ) -> Result<Option<DbRow>, DbRuntimeError> {
        tokio::time::timeout(Duration::from_millis(self.query_timeout_ms), async {
            let (sql, params) = statement.parts();
            let sql = prepare_sql(sql, self.backend, params)?;
            let row = bind_params(sqlx::query::<Any>(AssertSqlSafe(sql)), params, self.backend)?
                .fetch_optional(&mut **transaction)
                .await
                .map_err(|error| backend_operation_error(error, query_error()))?;
            row.as_ref().map(decode_row).transpose()
        })
        .await
        .map_err(|_| query_timeout_error())?
    }

    pub(crate) async fn commit_transaction(
        transaction: Transaction<'static, Any>,
    ) -> Result<(), DbRuntimeError> {
        transaction
            .commit()
            .await
            .map_err(|error| backend_operation_error(error, transaction_error()))
    }

    pub(crate) async fn rollback_transaction(
        transaction: Transaction<'static, Any>,
    ) -> Result<(), DbRuntimeError> {
        transaction
            .rollback()
            .await
            .map_err(|error| backend_operation_error(error, transaction_error()))
    }

    pub(crate) async fn open(
        backend: ServerBackend,
        connection_string: &str,
        options: ServerPoolOptions,
        ca_file: Option<&str>,
        cert_file: Option<&str>,
        key_file: Option<&str>,
    ) -> Result<Self, DbRuntimeError> {
        for path in [ca_file, cert_file, key_file].into_iter().flatten() {
            let file = std::fs::File::open(path).map_err(|_| invalid_tls_configuration())?;
            if !file
                .metadata()
                .map_err(|_| invalid_tls_configuration())?
                .is_file()
            {
                return Err(invalid_tls_configuration());
            }
        }
        if cert_file.is_some() != key_file.is_some() {
            return Err(invalid_tls_configuration());
        }
        let mut connection_string =
            secure_connection_string(backend, connection_string, ca_file, cert_file, key_file)?;
        sqlx::any::install_default_drivers();
        let opened = tokio::time::timeout(
            Duration::from_millis(options.connect_timeout_ms),
            AnyPoolOptions::new()
                .min_connections(0)
                .max_connections(options.max_connections)
                .idle_timeout(Some(Duration::from_millis(options.idle_timeout_ms)))
                .max_lifetime(Some(Duration::from_millis(options.max_lifetime_ms)))
                .acquire_timeout(Duration::from_millis(options.acquire_timeout_ms))
                .after_connect(move |connection, _| {
                    Box::pin(async move {
                        let timezone_sql = match backend {
                            ServerBackend::Postgresql => "SET TIME ZONE 'UTC'",
                            ServerBackend::Mysql => "SET time_zone = '+00:00'",
                        };
                        sqlx::query(timezone_sql).execute(connection).await?;
                        Ok(())
                    })
                })
                .connect(&connection_string),
        )
        .await;
        use zeroize::Zeroize;
        connection_string.zeroize();
        let pool = opened
            .map_err(|_| connection_timeout_error())?
            .map_err(|error| backend_operation_error(error, connection_error()))?;
        Ok(Self {
            pool,
            backend,
            query_timeout_ms: options.query_timeout_ms,
        })
    }

    pub(crate) async fn execute(
        &self,
        statement: &DbStatement,
    ) -> Result<DbExecResult, DbRuntimeError> {
        let (sql, params) = statement.parts();
        let sql = prepare_sql(sql, self.backend, params)?;
        if self.backend == ServerBackend::Postgresql && sql.contains("AS \"__tyrus_generated_id\"")
        {
            let row = tokio::time::timeout(Duration::from_millis(self.query_timeout_ms), async {
                bind_params(sqlx::query::<Any>(AssertSqlSafe(sql)), params, self.backend)?
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|error| backend_operation_error(error, execute_error()))?
                    .ok_or_else(execute_error)
            })
            .await
            .map_err(|_| query_timeout_error())??;
            let generated_id = row
                .try_get::<i64, _>("__tyrus_generated_id")
                .map_err(|_| DbRuntimeError::decode())?;
            return Ok(DbExecResult {
                rows_affected: 1,
                last_insert_rowid: generated_id,
            });
        }
        let result = tokio::time::timeout(Duration::from_millis(self.query_timeout_ms), async {
            bind_params(sqlx::query::<Any>(AssertSqlSafe(sql)), params, self.backend)?
                .execute(&self.pool)
                .await
                .map_err(|error| backend_operation_error(error, execute_error()))
        })
        .await
        .map_err(|_| query_timeout_error())??;
        Ok(DbExecResult {
            rows_affected: result.rows_affected(),
            last_insert_rowid: result.last_insert_id().unwrap_or(0),
        })
    }

    pub(crate) async fn execute_transaction(
        &self,
        statements: &[DbStatement],
    ) -> Result<Vec<DbExecResult>, DbRuntimeError> {
        tokio::time::timeout(
            Duration::from_millis(self.query_timeout_ms),
            self.execute_transaction_inner(statements),
        )
        .await
        .map_err(|_| query_timeout_error())?
    }

    async fn execute_transaction_inner(
        &self,
        statements: &[DbStatement],
    ) -> Result<Vec<DbExecResult>, DbRuntimeError> {
        let mut connection = self
            .pool
            .acquire()
            .await
            .map_err(|error| backend_operation_error(error, transaction_error()))?;
        let mut transaction = connection
            .begin()
            .await
            .map_err(|error| backend_operation_error(error, transaction_error()))?;
        let result = async {
            let mut results = Vec::with_capacity(statements.len());
            for statement in statements {
                let (sql, params) = statement.parts();
                let sql = prepare_sql(sql, self.backend, params)?;
                if self.backend == ServerBackend::Postgresql
                    && sql.contains("AS \"__tyrus_generated_id\"")
                {
                    let row =
                        bind_params(sqlx::query::<Any>(AssertSqlSafe(sql)), params, self.backend)?
                            .fetch_optional(&mut *transaction)
                            .await
                            .map_err(|error| backend_operation_error(error, execute_error()))?
                            .ok_or_else(execute_error)?;
                    let generated_id = row
                        .try_get::<i64, _>("__tyrus_generated_id")
                        .map_err(|_| DbRuntimeError::decode())?;
                    results.push(DbExecResult {
                        rows_affected: 1,
                        last_insert_rowid: generated_id,
                    });
                } else {
                    let result =
                        bind_params(sqlx::query::<Any>(AssertSqlSafe(sql)), params, self.backend)?
                            .execute(&mut *transaction)
                            .await
                            .map_err(|error| backend_operation_error(error, execute_error()))?;
                    results.push(DbExecResult {
                        rows_affected: result.rows_affected(),
                        last_insert_rowid: result.last_insert_id().unwrap_or(0),
                    });
                }
            }
            Ok(results)
        }
        .await;
        match result {
            Ok(results) => {
                transaction
                    .commit()
                    .await
                    .map_err(|error| backend_operation_error(error, transaction_error()))?;
                Ok(results)
            }
            Err(error) => {
                let _ = transaction.rollback().await;
                Err(error)
            }
        }
    }

    pub(crate) async fn query_many(
        &self,
        statement: &DbStatement,
    ) -> Result<Vec<DbRow>, DbRuntimeError> {
        let (sql, params) = statement.parts();
        let sql = prepare_sql(sql, self.backend, params)?;
        let rows = tokio::time::timeout(Duration::from_millis(self.query_timeout_ms), async {
            bind_params(sqlx::query::<Any>(AssertSqlSafe(sql)), params, self.backend)?
                .fetch_all(&self.pool)
                .await
                .map_err(|error| backend_operation_error(error, query_error()))
        })
        .await
        .map_err(|_| query_timeout_error())??;
        rows.iter().map(decode_row).collect()
    }

    pub(crate) async fn query_one(
        &self,
        statement: &DbStatement,
    ) -> Result<Option<DbRow>, DbRuntimeError> {
        let (sql, params) = statement.parts();
        let sql = prepare_sql(sql, self.backend, params)?;
        let row = tokio::time::timeout(Duration::from_millis(self.query_timeout_ms), async {
            bind_params(sqlx::query::<Any>(AssertSqlSafe(sql)), params, self.backend)?
                .fetch_optional(&self.pool)
                .await
                .map_err(|error| backend_operation_error(error, query_error()))
        })
        .await
        .map_err(|_| query_timeout_error())??;
        row.as_ref().map(decode_row).transpose()
    }

    pub(crate) async fn query_page(
        &self,
        rows_statement: &DbStatement,
        count_statement: &DbStatement,
    ) -> Result<(Vec<DbRow>, u64), DbRuntimeError> {
        tokio::time::timeout(
            Duration::from_millis(self.query_timeout_ms),
            self.query_page_inner(rows_statement, count_statement),
        )
        .await
        .map_err(|_| query_timeout_error())?
    }

    async fn query_page_inner(
        &self,
        rows_statement: &DbStatement,
        count_statement: &DbStatement,
    ) -> Result<(Vec<DbRow>, u64), DbRuntimeError> {
        let (rows_sql, rows_params) = rows_statement.parts();
        let (count_sql, count_params) = count_statement.parts();
        let rows_sql = prepare_sql(rows_sql, self.backend, rows_params)?;
        let count_sql = prepare_sql(count_sql, self.backend, count_params)?;
        let mut connection = self
            .pool
            .acquire()
            .await
            .map_err(|error| backend_operation_error(error, transaction_error()))?;
        if self.backend == ServerBackend::Mysql {
            sqlx::query::<Any>("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
                .execute(&mut *connection)
                .await
                .map_err(|error| backend_operation_error(error, transaction_error()))?;
        }
        let mut transaction = connection
            .begin()
            .await
            .map_err(|error| backend_operation_error(error, transaction_error()))?;
        let result = async {
            if self.backend == ServerBackend::Postgresql {
                sqlx::query::<Any>("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
                    .execute(&mut *transaction)
                    .await
                    .map_err(|error| backend_operation_error(error, transaction_error()))?;
            }
            let count_row = bind_params(
                sqlx::query::<Any>(AssertSqlSafe(count_sql)),
                count_params,
                self.backend,
            )?
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| backend_operation_error(error, query_error()))?
            .ok_or_else(DbRuntimeError::decode)?;
            let total = decode_count(&count_row)?;
            let rows = bind_params(
                sqlx::query::<Any>(AssertSqlSafe(rows_sql)),
                rows_params,
                self.backend,
            )?
            .fetch_all(&mut *transaction)
            .await
            .map_err(|error| backend_operation_error(error, query_error()))?;
            let rows = rows.iter().map(decode_row).collect::<Result<Vec<_>, _>>()?;
            Ok((rows, total))
        }
        .await;
        match result {
            Ok(value) => {
                transaction
                    .commit()
                    .await
                    .map_err(|error| backend_operation_error(error, transaction_error()))?;
                Ok(value)
            }
            Err(error) => {
                let _ = transaction.rollback().await;
                Err(error)
            }
        }
    }
}

fn secure_connection_string(
    backend: ServerBackend,
    connection_string: &str,
    ca_file: Option<&str>,
    cert_file: Option<&str>,
    key_file: Option<&str>,
) -> Result<String, DbRuntimeError> {
    if cert_file.is_some() != key_file.is_some() {
        return Err(invalid_tls_configuration());
    }
    let Ok(options) = connection_string.parse::<sqlx::any::AnyConnectOptions>() else {
        return Err(invalid_connection_error());
    };
    let mut url = options.database_url;
    let valid_scheme = match backend {
        ServerBackend::Postgresql => matches!(url.scheme(), "postgres" | "postgresql"),
        ServerBackend::Mysql => url.scheme() == "mysql",
    };
    if !valid_scheme || !url.host_str().is_some_and(|host| !host.is_empty()) {
        return Err(invalid_connection_error());
    }

    // Database TLS is opt-in. Keep driver and URI defaults when no TLS files
    // were configured; this supports local development databases without TLS.
    if ca_file.is_none() && cert_file.is_none() && key_file.is_none() {
        let parameter = match backend {
            ServerBackend::Postgresql => "sslmode",
            ServerBackend::Mysql => "ssl-mode",
        };
        if url
            .query_pairs()
            .filter(|(name, _)| name.eq_ignore_ascii_case(parameter))
            .count()
            > 1
        {
            return Err(invalid_tls_configuration());
        }
        return Ok(url.into());
    }

    let (parameter, required_mode) = match backend {
        ServerBackend::Postgresql => ("sslmode", "verify-full"),
        ServerBackend::Mysql => ("ssl-mode", "VERIFY_IDENTITY"),
    };
    let modes = url
        .query_pairs()
        .filter(|(name, _)| name.eq_ignore_ascii_case(parameter))
        .map(|(_, value)| value.into_owned())
        .collect::<Vec<_>>();
    match modes.as_slice() {
        [] => {
            url.query_pairs_mut().append_pair(parameter, required_mode);
        }
        [mode] if mode.eq_ignore_ascii_case(required_mode) => {}
        _ => return Err(invalid_tls_configuration()),
    }
    if let Some(ca_file) = ca_file {
        let ca_parameter = match backend {
            ServerBackend::Postgresql => "sslrootcert",
            ServerBackend::Mysql => "ssl-ca",
        };
        if url
            .query_pairs()
            .any(|(name, _)| name.eq_ignore_ascii_case(ca_parameter))
            || ca_file.is_empty()
        {
            return Err(invalid_tls_configuration());
        }
        url.query_pairs_mut().append_pair(ca_parameter, ca_file);
    }
    let (cert_parameter, key_parameter) = match backend {
        ServerBackend::Postgresql => ("sslcert", "sslkey"),
        ServerBackend::Mysql => ("ssl-cert", "ssl-key"),
    };
    let has_client_certificate_parameter = url.query_pairs().any(|(name, _)| {
        name.eq_ignore_ascii_case(cert_parameter) || name.eq_ignore_ascii_case(key_parameter)
    });
    if has_client_certificate_parameter {
        return Err(invalid_tls_configuration());
    }
    if let (Some(cert_file), Some(key_file)) = (cert_file, key_file) {
        url.query_pairs_mut()
            .append_pair(cert_parameter, cert_file)
            .append_pair(key_parameter, key_file);
    }
    Ok(url.into())
}

fn invalid_connection_error() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_INVALID_CONNECTION",
        message: "connection variable does not contain a valid URL for the configured backend",
    }
}

fn invalid_tls_configuration() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_TLS_CONFIGURATION",
        message: "configured server database TLS requires certificate and hostname verification",
    }
}

fn bind_params<'q>(
    mut query: sqlx::query::Query<'q, Any, sqlx::any::AnyArguments>,
    values: &[DbValue],
    backend: ServerBackend,
) -> Result<sqlx::query::Query<'q, Any, sqlx::any::AnyArguments>, DbRuntimeError> {
    for value in values {
        query = match value {
            DbValue::Null => query.bind(Option::<String>::None),
            DbValue::Bool(value) => query.bind(*value),
            DbValue::Signed(value) => query.bind(*value),
            DbValue::Unsigned(value) if backend == ServerBackend::Mysql => {
                query.bind(value.to_string())
            }
            DbValue::Unsigned(value) => {
                query.bind(i64::try_from(*value).map_err(|_| DbRuntimeError {
                    code: "ERR_DB_VALUE_OUT_OF_RANGE",
                    message: "unsigned integer exceeds the PostgreSQL driver integer range",
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
                if backend == ServerBackend::Mysql && nanoseconds % 1_000_000 != 0 {
                    return Err(DbRuntimeError {
                        code: "ERR_DB_VALUE_OUT_OF_RANGE",
                        message: "MySQL DATETIME(3) cannot store sub-millisecond Instant precision",
                    });
                }
                if backend == ServerBackend::Postgresql && nanoseconds % 1_000 != 0 {
                    return Err(DbRuntimeError {
                        code: "ERR_DB_VALUE_OUT_OF_RANGE",
                        message: "PostgreSQL TIMESTAMPTZ cannot store sub-microsecond Instant precision",
                    });
                }
                let text = timestamp.to_string();
                let text = if backend == ServerBackend::Mysql {
                    text.trim_end_matches('Z').replace('T', " ")
                } else {
                    text
                };
                query.bind(text)
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

fn decode_row(row: &sqlx::any::AnyRow) -> Result<DbRow, DbRuntimeError> {
    let mut columns = Vec::with_capacity(row.len());
    let mut values = Vec::with_capacity(row.len());
    for index in 0..row.len() {
        columns.push(row.column(index).name().to_string());
        let raw = row
            .try_get_raw(index)
            .map_err(|_| DbRuntimeError::decode())?;
        let info = raw.type_info();
        let value = if raw.is_null() {
            DbValue::Null
        } else {
            match info.kind() {
                AnyTypeInfoKind::Bool => DbValue::Bool(
                    row.try_get::<bool, _>(index)
                        .map_err(|_| DbRuntimeError::decode())?,
                ),
                AnyTypeInfoKind::SmallInt => DbValue::Signed(
                    row.try_get::<i16, _>(index)
                        .map_err(|_| DbRuntimeError::decode())? as i64,
                ),
                AnyTypeInfoKind::Integer => DbValue::Signed(
                    row.try_get::<i32, _>(index)
                        .map_err(|_| DbRuntimeError::decode())? as i64,
                ),
                AnyTypeInfoKind::BigInt => DbValue::Signed(
                    row.try_get::<i64, _>(index)
                        .map_err(|_| DbRuntimeError::decode())?,
                ),
                AnyTypeInfoKind::Real => DbValue::Real(
                    row.try_get::<f32, _>(index)
                        .map_err(|_| DbRuntimeError::decode())? as f64,
                ),
                AnyTypeInfoKind::Double => DbValue::Real(
                    row.try_get::<f64, _>(index)
                        .map_err(|_| DbRuntimeError::decode())?,
                ),
                AnyTypeInfoKind::Text => DbValue::Text(
                    row.try_get::<String, _>(index)
                        .map_err(|_| DbRuntimeError::decode())?,
                ),
                AnyTypeInfoKind::Blob => DbValue::Blob(
                    row.try_get::<Vec<u8>, _>(index)
                        .map_err(|_| DbRuntimeError::decode())?,
                ),
                AnyTypeInfoKind::Null => DbValue::Null,
            }
        };
        values.push(value);
    }
    Ok(DbRow::with_columns(columns, values))
}

fn decode_count(row: &sqlx::any::AnyRow) -> Result<u64, DbRuntimeError> {
    let row = decode_row(row)?;
    match row.values().first() {
        Some(DbValue::Signed(value)) => u64::try_from(*value).map_err(|_| DbRuntimeError::decode()),
        Some(DbValue::Unsigned(value)) => Ok(*value),
        _ => Err(DbRuntimeError::decode()),
    }
}

fn prepare_sql(
    sql: &str,
    backend: ServerBackend,
    params: &[DbValue],
) -> Result<String, DbRuntimeError> {
    reject_sqlite_relation_include(sql)?;
    let bytes = sql.as_bytes();
    let mut output = Vec::with_capacity(sql.len() + 8);
    let (mut index, mut parameter) = (0, 0usize);
    let mut state = SqlLexState::Normal;
    let mut dollar_tag = Vec::new();
    while index < bytes.len() {
        let byte = bytes[index];
        match state {
            SqlLexState::Normal => match byte {
                b'e' | b'E'
                    if backend == ServerBackend::Postgresql
                        && bytes.get(index + 1) == Some(&b'\'') =>
                {
                    output.extend_from_slice(&bytes[index..index + 2]);
                    state = SqlLexState::EscapeSingleQuote;
                    index += 2;
                }
                b'\'' => {
                    output.push(b'\'');
                    state = SqlLexState::SingleQuote;
                    index += 1;
                }
                b'"' => {
                    output.push(if backend == ServerBackend::Mysql {
                        b'`'
                    } else {
                        b'"'
                    });
                    state = SqlLexState::DoubleQuote;
                    index += 1;
                }
                b'`' if backend == ServerBackend::Mysql => {
                    output.push(b'`');
                    state = SqlLexState::BacktickQuote;
                    index += 1;
                }
                b'#' if backend == ServerBackend::Mysql => {
                    output.push(b'#');
                    state = SqlLexState::LineComment;
                    index += 1;
                }
                b'$' => {
                    if backend == ServerBackend::Postgresql {
                        if let Some((tag, length)) = dollar_quote_tag(&bytes[index..]) {
                            output.extend_from_slice(tag);
                            dollar_tag.clear();
                            dollar_tag.extend_from_slice(tag);
                            state = SqlLexState::DollarQuote;
                            index += length;
                        } else {
                            output.push(byte);
                            index += 1;
                        }
                    } else {
                        output.push(byte);
                        index += 1;
                    }
                }
                b'-' if bytes.get(index + 1) == Some(&b'-')
                    && (backend == ServerBackend::Postgresql
                        || bytes.get(index + 2).is_some_and(|byte| {
                            byte.is_ascii_whitespace() || byte.is_ascii_control()
                        })) =>
                {
                    output.extend_from_slice(b"--");
                    state = SqlLexState::LineComment;
                    index += 2;
                }
                b'/' if bytes.get(index + 1) == Some(&b'*') => {
                    output.extend_from_slice(b"/*");
                    state = SqlLexState::BlockComment;
                    index += 2;
                }
                b'?' => {
                    parameter += 1;
                    let uuid_parameter = sql[index..].starts_with(UUID_PARAMETER_MARKER);
                    match (backend, uuid_parameter) {
                        (ServerBackend::Postgresql, true) => {
                            output.extend_from_slice(b"CAST($");
                            output.extend_from_slice(parameter.to_string().as_bytes());
                            output.extend_from_slice(b" AS UUID)");
                        }
                        (ServerBackend::Postgresql, false) => {
                            if matches!(params.get(parameter - 1), Some(DbValue::Instant { .. })) {
                                output.extend_from_slice(b"CAST($");
                                output.extend_from_slice(parameter.to_string().as_bytes());
                                output.extend_from_slice(b" AS TIMESTAMPTZ)");
                            } else {
                                output.push(b'$');
                                output.extend_from_slice(parameter.to_string().as_bytes());
                            }
                        }
                        (ServerBackend::Mysql, true) => output.push(b'?'),
                        (ServerBackend::Mysql, false) => {
                            if matches!(params.get(parameter - 1), Some(DbValue::Instant { .. })) {
                                output.extend_from_slice(b"CAST(? AS DATETIME(3))");
                            } else {
                                output.push(b'?');
                            }
                        }
                    }
                    if uuid_parameter {
                        index += UUID_PARAMETER_MARKER.len();
                    } else {
                        index += 1;
                    }
                }
                _ => {
                    output.push(byte);
                    index += 1;
                }
            },
            SqlLexState::SingleQuote => {
                output.push(byte);
                index += 1;
                if byte == b'\\' && backend == ServerBackend::Mysql {
                    if let Some(escaped) = bytes.get(index) {
                        output.push(*escaped);
                        index += 1;
                    }
                } else if byte == b'\'' {
                    if bytes.get(index) == Some(&b'\'') {
                        output.push(b'\'');
                        index += 1;
                    } else {
                        state = SqlLexState::Normal;
                    }
                }
            }
            SqlLexState::EscapeSingleQuote => {
                output.push(byte);
                index += 1;
                if byte == b'\\' {
                    if let Some(escaped) = bytes.get(index) {
                        output.push(*escaped);
                        index += 1;
                    }
                } else if byte == b'\'' {
                    if bytes.get(index) == Some(&b'\'') {
                        output.push(b'\'');
                        index += 1;
                    } else {
                        state = SqlLexState::Normal;
                    }
                }
            }
            SqlLexState::DoubleQuote => {
                output.push(if backend == ServerBackend::Mysql && byte == b'"' {
                    b'`'
                } else {
                    byte
                });
                index += 1;
                if byte == b'"' {
                    if bytes.get(index) == Some(&b'"') {
                        output.push(if backend == ServerBackend::Mysql {
                            b'`'
                        } else {
                            b'"'
                        });
                        index += 1;
                    } else {
                        state = SqlLexState::Normal;
                    }
                }
            }
            SqlLexState::LineComment => {
                output.push(byte);
                index += 1;
                if byte == b'\n' {
                    state = SqlLexState::Normal;
                }
            }
            SqlLexState::BlockComment => {
                output.push(byte);
                index += 1;
                if byte == b'*' && bytes.get(index) == Some(&b'/') {
                    output.push(b'/');
                    index += 1;
                    state = SqlLexState::Normal;
                }
            }
            SqlLexState::DollarQuote => {
                if bytes[index..].starts_with(&dollar_tag) {
                    output.extend_from_slice(&dollar_tag);
                    index += dollar_tag.len();
                    state = SqlLexState::Normal;
                } else {
                    output.push(byte);
                    index += 1;
                }
            }
            SqlLexState::BacktickQuote => {
                output.push(byte);
                index += 1;
                if byte == b'`' {
                    if bytes.get(index) == Some(&b'`') {
                        output.push(b'`');
                        index += 1;
                    } else {
                        state = SqlLexState::Normal;
                    }
                }
            }
        }
    }
    if matches!(
        state,
        SqlLexState::SingleQuote
            | SqlLexState::EscapeSingleQuote
            | SqlLexState::DoubleQuote
            | SqlLexState::DollarQuote
            | SqlLexState::BacktickQuote
    ) {
        return Err(DbRuntimeError {
            code: "ERR_DB_INVALID_SQL",
            message: "SQL contains an unterminated quoted value",
        });
    }
    String::from_utf8(output).map_err(|_| DbRuntimeError {
        code: "ERR_DB_INVALID_SQL",
        message: "SQL could not be encoded for the configured server backend",
    })
}

fn reject_sqlite_relation_include(sql: &str) -> Result<(), DbRuntimeError> {
    if sql.contains("\"__tyrus_json__") {
        return Err(DbRuntimeError {
            code: "ERR_DB_FEATURE_UNSUPPORTED",
            message: "relation includes are currently supported only by the SQLite backend",
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SqlLexState {
    Normal,
    SingleQuote,
    EscapeSingleQuote,
    DoubleQuote,
    LineComment,
    BlockComment,
    DollarQuote,
    BacktickQuote,
}

fn dollar_quote_tag(source: &[u8]) -> Option<(&[u8], usize)> {
    if source.first() != Some(&b'$') {
        return None;
    }
    let closing = source.iter().skip(1).position(|byte| *byte == b'$')? + 1;
    let tag = &source[..=closing];
    if tag[1..closing]
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        Some((tag, tag.len()))
    } else {
        None
    }
}

fn connection_error() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_CONNECTION_FAILED",
        message: "server database connection failed",
    }
}

fn connection_timeout_error() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_CONNECTION_TIMEOUT",
        message: "server database connection exceeded the configured timeout",
    }
}

fn pool_timeout_error() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_POOL_TIMEOUT",
        message: "server database pool could not provide a connection before the configured timeout",
    }
}

fn query_timeout_error() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_QUERY_TIMEOUT",
        message: "server database operation exceeded the configured query timeout",
    }
}

fn backend_operation_error(error: sqlx::Error, fallback: DbRuntimeError) -> DbRuntimeError {
    match error {
        sqlx::Error::PoolTimedOut => pool_timeout_error(),
        sqlx::Error::Database(_database_error) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "database backend error [{}]: {}",
                _database_error.code().as_deref().unwrap_or("unknown"),
                _database_error.message(),
            );
            fallback
        }
        _ => fallback,
    }
}

fn execute_error() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_EXECUTE_FAILED",
        message: "server database mutation failed",
    }
}

fn query_error() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_QUERY_FAILED",
        message: "server database query failed",
    }
}

fn transaction_error() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_TRANSACTION_FAILED",
        message: "server database transaction failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn statement(sql: String, parameters: Vec<DbValue>) -> DbStatement {
        DbStatement::new(sql.into_boxed_str(), parameters)
    }

    async fn exercise_server_backend(
        backend: ServerBackend,
        connection_string: &str,
    ) -> Result<(), DbRuntimeError> {
        let pool = ServerPoolOptions::default();
        let database =
            ServerDatabase::open(backend, connection_string, pool, None, None, None).await?;
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let table = format!("tyrus_db_check_{}_{}", std::process::id(), suffix);
        let identifier_table = format!("{table}_ids");
        let (create_sql, query_sql, temporal_insert, temporal_match_sql) = match backend {
            ServerBackend::Postgresql => (
                format!(
                    "CREATE TABLE {table} (id BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY, label TEXT NOT NULL UNIQUE, note TEXT NOT NULL DEFAULT 'default note', optional_note TEXT NULL, occurred_at TIMESTAMPTZ NOT NULL)"
                ),
                format!("SELECT id, label, note, optional_note FROM {table} WHERE id = ?"),
                format!(
                    "INSERT INTO {table} (label, occurred_at) VALUES (?, ?) RETURNING id AS \"__tyrus_generated_id\""
                ),
                format!(
                    "UPDATE {table} SET label = 'instant matched' WHERE occurred_at = ? AND id = ?"
                ),
            ),
            ServerBackend::Mysql => (
                format!(
                    "CREATE TABLE {table} (id BIGINT AUTO_INCREMENT PRIMARY KEY, label VARCHAR(255) NOT NULL UNIQUE, note VARCHAR(255) NOT NULL DEFAULT 'default note', optional_note VARCHAR(255) NULL, occurred_at DATETIME(3) NOT NULL)"
                ),
                format!("SELECT id, label, note, optional_note FROM {table} WHERE id = ?"),
                format!("INSERT INTO {table} (label, occurred_at) VALUES (?, ?)"),
                format!(
                    "UPDATE {table} SET label = 'instant matched' WHERE occurred_at = ? AND id = ?"
                ),
            ),
        };
        let result = async {
            database.execute(&statement(create_sql, Vec::new())).await?;
            let inserted = database
                .execute(&statement(
                    temporal_insert.clone(),
                    vec![
                        DbValue::Text("runtime integration".to_string()),
                        DbValue::Instant {
                            seconds: 1_735_689_599,
                            nanoseconds: 999_000_000,
                        },
                    ],
                ))
                .await?;
            if inserted.rows_affected != 1 || inserted.last_insert_rowid <= 0 {
                return Err(DbRuntimeError::decode());
            }
            let found = database
                .query_one(&statement(
                    query_sql.clone(),
                    vec![DbValue::Signed(inserted.last_insert_rowid)],
                ))
                .await?
                .ok_or_else(DbRuntimeError::decode)?;
            if found.values()
                != [
                    DbValue::Signed(inserted.last_insert_rowid),
                    DbValue::Text("runtime integration".to_string()),
                    DbValue::Text("default note".to_string()),
                    DbValue::Null,
                ]
            {
                return Err(DbRuntimeError::decode());
            }
            let temporal = database
                .execute(&statement(
                    temporal_match_sql,
                    vec![
                        DbValue::Instant {
                            seconds: 1_735_689_599,
                            nanoseconds: 999_000_000,
                        },
                        DbValue::Signed(inserted.last_insert_rowid),
                    ],
                ))
                .await?;
            if temporal.rows_affected != 1 {
                return Err(DbRuntimeError::decode());
            }
            let unsupported_precision = database
                .execute(&statement(
                    temporal_insert.clone(),
                    vec![
                        DbValue::Text("unsupported precision".to_string()),
                        DbValue::Instant {
                            seconds: 1_735_689_600,
                            nanoseconds: 1,
                        },
                    ],
                ))
                .await
                .unwrap_err();
            if unsupported_precision.code != "ERR_DB_VALUE_OUT_OF_RANGE" {
                return Err(DbRuntimeError::decode());
            }

            let mut transaction = database.begin_transaction().await?;
            let failed = database
                .execute_in_transaction(
                    &mut transaction,
                    &statement(
                        temporal_insert.clone(),
                        vec![
                            DbValue::Text("runtime integration".to_string()),
                            DbValue::Instant {
                                seconds: 1_735_689_600,
                                nanoseconds: 0,
                            },
                        ],
                    ),
                )
                .await
                .unwrap_err();
            if failed.code != "ERR_DB_EXECUTE_FAILED" {
                return Err(DbRuntimeError::decode());
            }
            ServerDatabase::rollback_transaction(transaction).await?;
            let rows = database
                .query_many(&statement(
                    format!("SELECT label FROM {table} ORDER BY id"),
                    Vec::new(),
                ))
                .await?;
            if rows.len() != 1 {
                return Err(DbRuntimeError::decode());
            }

            let count = database
                .query_one(&statement(
                    format!("SELECT COUNT(*) AS total FROM {table}"),
                    Vec::new(),
                ))
                .await?
                .ok_or_else(DbRuntimeError::decode)?;
            if count.values() != [DbValue::Signed(1)] {
                return Err(DbRuntimeError::decode());
            }

            let (rows_sql, count_sql) = (
                format!("SELECT id FROM {table} ORDER BY id LIMIT ? OFFSET ?"),
                format!("SELECT COUNT(*) FROM {table}"),
            );
            let (page, total) = database
                .query_page(
                    &statement(rows_sql, vec![DbValue::Signed(10), DbValue::Signed(0)]),
                    &statement(count_sql, Vec::new()),
                )
                .await?;
            if page.len() != 1 || total != 1 {
                return Err(DbRuntimeError::decode());
            }

            let (identifier_schema, insert_identifier, select_identifiers, lookup_uuid) =
                match backend {
                    ServerBackend::Postgresql => {
                        let version = database
                            .query_one(&statement(
                                "SELECT current_setting('server_version_num')::BIGINT".into(),
                                Vec::new(),
                            ))
                            .await?
                            .and_then(|row| match row.values().first() {
                                Some(DbValue::Signed(version)) => Some(*version),
                                _ => None,
                            })
                            .ok_or_else(DbRuntimeError::decode)?;
                        let (uuid7_column, uuid7_projection) = if version >= 180_000 {
                            (
                                ", uuid7_id UUID NOT NULL DEFAULT uuidv7()",
                                ", CAST(uuid7_id AS TEXT) AS uuid7_id",
                            )
                        } else {
                            ("", "")
                        };
                        (
                            format!("CREATE TABLE {identifier_table} (uuid_id UUID NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY{uuid7_column})"),
                            format!("INSERT INTO {identifier_table} DEFAULT VALUES"),
                            format!("SELECT CAST(uuid_id AS TEXT) AS uuid_id{uuid7_projection} FROM {identifier_table}"),
                            format!("SELECT CAST(uuid_id AS TEXT) FROM {identifier_table} WHERE uuid_id = ?/*__tyrus_uuid_parameter__*/"),
                        )
                    }
                    ServerBackend::Mysql => (
                        format!("CREATE TABLE {identifier_table} (uuid_id CHAR(36) NOT NULL DEFAULT (UUID()) PRIMARY KEY, short_id BIGINT UNSIGNED NOT NULL DEFAULT (UUID_SHORT()) UNIQUE)"),
                        format!("INSERT INTO {identifier_table} () VALUES ()"),
                        format!("SELECT uuid_id, CAST(short_id AS CHAR) AS short_id FROM {identifier_table}"),
                        format!("SELECT uuid_id FROM {identifier_table} WHERE uuid_id = ?/*__tyrus_uuid_parameter__*/"),
                    ),
                };
            database
                .execute(&statement(identifier_schema, Vec::new()))
                .await?;
            database
                .execute(&statement(insert_identifier, Vec::new()))
                .await?;
            let identifier_row = database
                .query_one(&statement(select_identifiers, Vec::new()))
                .await?
                .ok_or_else(DbRuntimeError::decode)?;
            let uuid = match identifier_row.values().first() {
                Some(DbValue::Text(uuid)) if is_canonical_uuid(uuid) => uuid.clone(),
                _ => return Err(DbRuntimeError::decode()),
            };
            if backend == ServerBackend::Mysql {
                if uuid.as_bytes().get(14) != Some(&b'1') {
                    return Err(DbRuntimeError::decode());
                }
                let short_id = match identifier_row.values().get(1) {
                    Some(DbValue::Text(value)) => value
                        .parse::<u64>()
                        .map_err(|_| DbRuntimeError::decode())?,
                    _ => return Err(DbRuntimeError::decode()),
                };
                if short_id == 0 {
                    return Err(DbRuntimeError::decode());
                }
            } else {
                if uuid.as_bytes().get(14) != Some(&b'4') {
                    return Err(DbRuntimeError::decode());
                }
                if identifier_row.values().get(1).is_some() {
                    let uuid7 = match identifier_row.values().get(1) {
                        Some(DbValue::Text(value)) if is_canonical_uuid(value) => value,
                        _ => return Err(DbRuntimeError::decode()),
                    };
                    if uuid7.as_bytes().get(14) != Some(&b'7') {
                        return Err(DbRuntimeError::decode());
                    }
                }
            }
            database
                .execute(&statement(
                    format!(
                        "UPDATE {identifier_table} SET uuid_id = ?/*__tyrus_uuid_parameter__*/ WHERE uuid_id = ?/*__tyrus_uuid_parameter__*/"
                    ),
                    vec![DbValue::Text(uuid.clone()), DbValue::Text(uuid.clone())],
                ))
                .await?;
            let matched_uuid = database
                .query_one(&statement(lookup_uuid, vec![DbValue::Text(uuid)]))
                .await?
                .ok_or_else(DbRuntimeError::decode)?;
            if matched_uuid.values().len() != 1 {
                return Err(DbRuntimeError::decode());
            }
            Ok(())
        }
        .await;
        let identifier_cleanup = database
            .execute(&statement(
                format!("DROP TABLE IF EXISTS {identifier_table}"),
                Vec::new(),
            ))
            .await;
        let cleanup = database
            .execute(&statement(
                format!("DROP TABLE IF EXISTS {table}"),
                Vec::new(),
            ))
            .await;
        database.close().await;
        result?;
        identifier_cleanup.map_err(|_| DbRuntimeError::decode())?;
        cleanup.map(|_| ())
    }

    #[test]
    fn server_sql_rewrites_only_placeholders_outside_literals_and_comments() {
        let sql = "SELECT ?, '?' AS literal, \"?\" AS identifier, $$?$$ AS body, -- ?\n ?";
        let params = [DbValue::Signed(4), DbValue::Signed(9)];
        assert_eq!(
            prepare_sql(sql, ServerBackend::Postgresql, &params).unwrap(),
            "SELECT $1, '?' AS literal, \"?\" AS identifier, $$?$$ AS body, -- ?\n $2"
        );

        let mysql_sql =
            "SELECT ?, '?' AS literal, \"name\" AS identifier, `?` AS quoted, # ?\n ? /* ? */";
        assert_eq!(
            prepare_sql(mysql_sql, ServerBackend::Mysql, &params).unwrap(),
            "SELECT ?, '?' AS literal, `name` AS identifier, `?` AS quoted, # ?\n ? /* ? */"
        );
    }

    #[test]
    fn server_instant_placeholders_use_backend_native_temporal_types() {
        let instant = [DbValue::Instant {
            seconds: 1_735_689_600,
            nanoseconds: 123_000_000,
        }];
        assert_eq!(
            prepare_sql("SELECT ?", ServerBackend::Postgresql, &instant).unwrap(),
            "SELECT CAST($1 AS TIMESTAMPTZ)"
        );
        assert_eq!(
            prepare_sql("SELECT ?", ServerBackend::Mysql, &instant).unwrap(),
            "SELECT CAST(? AS DATETIME(3))"
        );
    }

    #[test]
    fn server_uuid_placeholders_cast_postgresql_parameters_and_strip_mysql_markers() {
        let uuid = [DbValue::Text(
            "0194f2b2-7b3a-7c01-8a53-76fe2c70603c".to_string(),
        )];
        assert_eq!(
            prepare_sql(
                "INSERT INTO users (id) VALUES (?/*__tyrus_uuid_parameter__*/)",
                ServerBackend::Postgresql,
                &uuid
            )
            .unwrap(),
            "INSERT INTO users (id) VALUES (CAST($1 AS UUID))"
        );
        assert_eq!(
            prepare_sql(
                "SELECT * FROM users WHERE id = ?/*__tyrus_uuid_parameter__*/",
                ServerBackend::Mysql,
                &uuid
            )
            .unwrap(),
            "SELECT * FROM users WHERE id = ?"
        );
    }

    #[test]
    fn mysql_accepts_full_u64_bind_values_while_postgresql_checks_its_signed_range() {
        let mysql = bind_params(
            sqlx::query::<Any>("SELECT ?"),
            &[DbValue::Unsigned(u64::MAX)],
            ServerBackend::Mysql,
        );
        assert!(mysql.is_ok());

        let postgresql = match bind_params(
            sqlx::query::<Any>("SELECT $1"),
            &[DbValue::Unsigned(u64::MAX)],
            ServerBackend::Postgresql,
        ) {
            Ok(_) => panic!("PostgreSQL accepted a u64 above its signed range"),
            Err(error) => error,
        };
        assert_eq!(postgresql.code, "ERR_DB_VALUE_OUT_OF_RANGE");
    }

    #[test]
    fn server_identifier_uuid_validation_requires_canonical_hex_shape() {
        assert!(is_canonical_uuid("6ccd780c-baba-1026-9564-5b8c656024db"));
        assert!(!is_canonical_uuid("6ccd780cbaba102695645b8c656024db"));
        assert!(!is_canonical_uuid("6ccd780c-baba-1026-9564-5b8c656024dg"));
    }

    #[test]
    fn server_sql_rejects_unterminated_literals_and_sqlite_only_relation_markers() {
        for sql in [
            "SELECT 'unfinished",
            "SELECT \"unfinished",
            "SELECT $tag$unfinished",
        ] {
            assert_eq!(
                prepare_sql(sql, ServerBackend::Postgresql, &[])
                    .unwrap_err()
                    .code,
                "ERR_DB_INVALID_SQL"
            );
        }
        assert_eq!(
            prepare_sql("SELECT \"__tyrus_json__users\"", ServerBackend::Mysql, &[])
                .unwrap_err()
                .code,
            "ERR_DB_FEATURE_UNSUPPORTED"
        );
    }

    #[test]
    fn server_connection_urls_leave_tls_optional_and_verify_it_when_configured() {
        let postgresql = secure_connection_string(
            ServerBackend::Postgresql,
            "postgresql://user:secret@localhost/app",
            None,
            None,
            None,
        )
        .unwrap();
        assert!(!postgresql.contains("sslmode="));

        let mysql = secure_connection_string(
            ServerBackend::Mysql,
            "mysql://user:secret@localhost/app",
            None,
            None,
            None,
        )
        .unwrap();
        assert!(!mysql.contains("ssl-mode="));

        for (backend, url) in [
            (
                ServerBackend::Postgresql,
                "mysql://user:secret@localhost/app",
            ),
            (ServerBackend::Mysql, "postgres://user:secret@localhost/app"),
        ] {
            assert!(secure_connection_string(backend, url, None, None, None).is_err());
        }

        let postgres_plaintext = secure_connection_string(
            ServerBackend::Postgresql,
            "postgres://user:secret@localhost/app?sslmode=disable",
            None,
            None,
            None,
        )
        .unwrap();
        assert!(postgres_plaintext.contains("sslmode=disable"));
        let mysql_plaintext = secure_connection_string(
            ServerBackend::Mysql,
            "mysql://user:secret@localhost/app?ssl-mode=DISABLED",
            None,
            None,
            None,
        )
        .unwrap();
        assert!(mysql_plaintext.contains("ssl-mode=DISABLED"));

        let postgres_verified = secure_connection_string(
            ServerBackend::Postgresql,
            "postgres://user:secret@localhost/app",
            Some("ca.pem"),
            None,
            None,
        )
        .unwrap();
        assert!(postgres_verified.contains("sslmode=verify-full"));

        for (backend, url, ca_file, cert_file, key_file) in [
            (
                ServerBackend::Postgresql,
                "postgres://user:secret@localhost/app?sslmode=verify-full&sslmode=require",
                None,
                None,
                None,
            ),
            (
                ServerBackend::Postgresql,
                "postgres://user:secret@localhost/app?sslrootcert=other.pem",
                Some("ca.pem"),
                None,
                None,
            ),
            (
                ServerBackend::Mysql,
                "mysql://user:secret@localhost/app?ssl-cert=other.pem",
                None,
                Some("client.pem"),
                Some("client.key"),
            ),
            (
                ServerBackend::Mysql,
                "mysql://user:secret@localhost/app",
                None,
                Some("client.pem"),
                None,
            ),
        ] {
            let error =
                secure_connection_string(backend, url, ca_file, cert_file, key_file).unwrap_err();
            assert_eq!(error.code, "ERR_DB_TLS_CONFIGURATION");
            assert!(!error.message.contains("secret"));
        }
    }

    #[test]
    fn server_pool_options_reject_unknown_keys_and_out_of_range_limits() {
        let defaults = ServerPoolOptions::from_json(&serde_json::json!({})).unwrap();
        assert!(defaults.validate());
        assert_eq!(defaults.max_connections, 10);
        for options in [
            serde_json::json!({"maxConnections": 0}),
            serde_json::json!({"maxConnections": 101}),
            serde_json::json!({"connectTimeoutMs": 0}),
            serde_json::json!({"connectTimeoutMs": 120_001}),
            serde_json::json!({"acquireTimeoutMs": 0}),
            serde_json::json!({"queryTimeoutMs": 120_001}),
            serde_json::json!({"idleTimeoutMs": 86_400_001}),
            serde_json::json!({"maxLifetimeMs": 0}),
            serde_json::json!({"maxLifetimeMs": 86_400_001}),
        ] {
            assert!(
                !ServerPoolOptions::from_json(&options)
                    .expect("known numeric pool option should parse")
                    .validate(),
                "accepted invalid pool options: {options}"
            );
        }
        for options in [
            serde_json::json!({"maxLifetime": 10}),
            serde_json::json!({"connectTimeoutMs": -1}),
            serde_json::json!({"queryTimeoutMs": 1.5}),
            serde_json::json!({"unknown": true}),
            serde_json::json!([]),
        ] {
            assert!(
                ServerPoolOptions::from_json(&options).is_none(),
                "accepted malformed pool options: {options}"
            );
        }
        assert!(
            ServerPoolOptions::from_json(&serde_json::json!({
                "maxConnections": 100,
                "connectTimeoutMs": 120_000,
                "acquireTimeoutMs": 120_000,
                "queryTimeoutMs": 120_000,
                "idleTimeoutMs": 86_400_000,
                "maxLifetimeMs": 86_400_000
            }))
            .unwrap()
            .validate()
        );
    }

    #[tokio::test]
    async fn server_env_backends_run_isolated_crud_transaction_and_page_checks() {
        for (backend, variable) in [
            (ServerBackend::Postgresql, "TYRUS_TEST_POSTGRESQL_URL"),
            (ServerBackend::Mysql, "TYRUS_TEST_MYSQL_URL"),
        ] {
            let Ok(connection_string) = std::env::var(variable) else {
                eprintln!("Skipping {backend:?} integration: {variable} is unset");
                continue;
            };
            exercise_server_backend(backend, &connection_string)
                .await
                .unwrap_or_else(|error| panic!("{backend:?} integration failed: {error:?}"));
        }
    }

    fn is_canonical_uuid(value: &str) -> bool {
        value.len() == 36
            && value.bytes().enumerate().all(|(index, byte)| {
                if matches!(index, 8 | 13 | 18 | 23) {
                    byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            })
    }
}
