use super::mongodb::{MongoDatabase, MongoPoolOptions, MongoReadOperation, MongoRuntimeError};
use super::server::{ServerBackend, ServerDatabase, ServerPoolOptions};
use super::sqlite::{DbExecResult, DbRuntimeError, SqliteDatabase, SqliteOptions};
use super::{DbRow, DbStatement, DbValue};
use sqlx::{Any, Sqlite, Transaction};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, mpsc};
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::task::JoinSet;

const DB_REQUEST_QUEUE_CAPACITY: usize = 128;
const DB_CONCURRENT_REQUEST_LIMIT: usize = 32;
static NEXT_TRANSACTION_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

enum Operation {
    Exec,
    One,
    Many,
    Page(DbStatement),
    #[allow(
        dead_code,
        reason = "transaction dispatch is prepared for the public callback API"
    )]
    Transaction(Vec<DbStatement>),
    TransactionStart,
    TransactionExec(u64),
    TransactionMutationReadback {
        id: u64,
        query: DbStatement,
        read_before: bool,
        use_inserted_id: bool,
    },
    TransactionOne(u64),
    TransactionMany(u64),
    TransactionFinish {
        id: u64,
        commit: bool,
    },
    Mongo(MongoReadOperation),
    MongoMutation,
    MutationReadback {
        query: DbStatement,
        read_before: bool,
        use_inserted_id: bool,
    },
}

pub(super) enum DbResponse {
    Exec(DbExecResult),
    One(Option<DbRow>),
    Many(Vec<DbRow>),
    Page {
        rows: Vec<DbRow>,
        total: u64,
    },
    #[allow(
        dead_code,
        reason = "transaction dispatch is prepared for the public callback API"
    )]
    Transaction(Vec<DbExecResult>),
    TransactionId(u64),
    TransactionFinished,
    Json(serde_json::Value),
}

enum ActiveTransaction {
    Sqlite(Transaction<'static, Sqlite>),
    Server(Transaction<'static, Any>),
}

struct TransactionEntry {
    target: String,
    transaction: ActiveTransaction,
}

enum TransactionDatabase<'a> {
    Sqlite(&'a SqliteDatabase),
    Server(&'a ServerDatabase),
    Unsupported,
}

struct Request {
    target: String,
    statement: DbStatement,
    operation: Operation,
    reply: mpsc::SyncSender<Result<DbResponse, DbRuntimeError>>,
}

pub(super) struct ReadbackOptions {
    pub(super) read_before: bool,
    pub(super) use_inserted_id: bool,
    pub(super) transaction_id: Option<u64>,
}

struct Service {
    sender: mpsc::SyncSender<Request>,
    shutdown: Arc<(Mutex<bool>, Condvar)>,
}

static SERVICE: OnceLock<Result<Service, DbRuntimeError>> = OnceLock::new();
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

pub(super) fn execute(
    path: String,
    sql: String,
    parameters: Vec<DbValue>,
) -> Result<DbExecResult, DbRuntimeError> {
    let DbResponse::Exec(result) = request(path, sql, parameters, Operation::Exec)? else {
        unreachable!("database worker returned the wrong operation")
    };
    Ok(result)
}

pub(super) fn mutate_with_readback(
    target: String,
    mutation_sql: String,
    mutation_parameters: Vec<DbValue>,
    read_sql: String,
    read_parameters: Vec<DbValue>,
    options: ReadbackOptions,
) -> Result<Option<DbRow>, DbRuntimeError> {
    let read_statement = DbStatement::new(read_sql.into(), read_parameters);
    let operation = if let Some(id) = options.transaction_id {
        Operation::TransactionMutationReadback {
            id,
            query: read_statement,
            read_before: options.read_before,
            use_inserted_id: options.use_inserted_id,
        }
    } else {
        Operation::MutationReadback {
            query: read_statement,
            read_before: options.read_before,
            use_inserted_id: options.use_inserted_id,
        }
    };
    let DbResponse::One(row) = request(target, mutation_sql, mutation_parameters, operation)?
    else {
        unreachable!("database worker returned the wrong mutation readback")
    };
    Ok(row)
}

pub(super) fn insert_many_with_readback(
    target: String,
    mutations: Vec<DbStatement>,
    readbacks: Vec<DbStatement>,
    use_inserted_ids: Vec<bool>,
    transaction_id: Option<u64>,
) -> Result<Vec<DbRow>, DbRuntimeError> {
    if mutations.is_empty()
        || mutations.len() != readbacks.len()
        || mutations.len() != use_inserted_ids.len()
    {
        return Err(DbRuntimeError::decode());
    }
    let owns_transaction = transaction_id.is_none();
    let id = match transaction_id {
        Some(id) => id,
        None => begin_transaction(target.clone())?,
    };
    let result = (|| {
        let mut rows = Vec::with_capacity(mutations.len());
        for ((mutation, readback), use_inserted_id) in
            mutations.into_iter().zip(readbacks).zip(use_inserted_ids)
        {
            let (mutation_sql, mutation_parameters) = mutation.into_parts();
            let execution = execute_in_transaction(
                target.clone(),
                id,
                mutation_sql.into_string(),
                mutation_parameters,
            )?;
            let readback = if use_inserted_id {
                let (sql, mut parameters) = readback.into_parts();
                parameters.push(DbValue::Signed(execution.last_insert_rowid));
                DbStatement::new(sql, parameters)
            } else {
                readback
            };
            let (read_sql, read_parameters) = readback.into_parts();
            let row = query_one_in_transaction(
                target.clone(),
                id,
                read_sql.into_string(),
                read_parameters,
            )?
            .ok_or_else(DbRuntimeError::decode)?;
            rows.push(row);
        }
        Ok(rows)
    })();
    if owns_transaction {
        match result {
            Ok(rows) => {
                finish_transaction(target, id, true)?;
                Ok(rows)
            }
            Err(error) => {
                let _ = finish_transaction(target, id, false);
                Err(error)
            }
        }
    } else {
        result
    }
}

#[allow(
    dead_code,
    reason = "transaction dispatch is prepared for the public callback API"
)]
pub(super) fn execute_transaction(
    target: String,
    statements: Vec<DbStatement>,
) -> Result<Vec<DbExecResult>, DbRuntimeError> {
    let DbResponse::Transaction(results) = request(
        target,
        String::new(),
        Vec::new(),
        Operation::Transaction(statements),
    )?
    else {
        unreachable!("database worker returned the wrong transaction result")
    };
    Ok(results)
}

pub(super) fn begin_transaction(target: String) -> Result<u64, DbRuntimeError> {
    let DbResponse::TransactionId(id) = request(
        target,
        String::new(),
        Vec::new(),
        Operation::TransactionStart,
    )?
    else {
        unreachable!("database worker returned the wrong transaction handle")
    };
    Ok(id)
}

pub(super) fn execute_in_transaction(
    target: String,
    id: u64,
    sql: String,
    parameters: Vec<DbValue>,
) -> Result<DbExecResult, DbRuntimeError> {
    let DbResponse::Exec(result) =
        request(target, sql, parameters, Operation::TransactionExec(id))?
    else {
        unreachable!("database worker returned the wrong transaction execution result")
    };
    Ok(result)
}

pub(super) fn query_one_in_transaction(
    target: String,
    id: u64,
    sql: String,
    parameters: Vec<DbValue>,
) -> Result<Option<DbRow>, DbRuntimeError> {
    let DbResponse::One(result) = request(target, sql, parameters, Operation::TransactionOne(id))?
    else {
        unreachable!("database worker returned the wrong transaction read result")
    };
    Ok(result)
}

pub(super) fn query_many_in_transaction(
    target: String,
    id: u64,
    sql: String,
    parameters: Vec<DbValue>,
) -> Result<Vec<DbRow>, DbRuntimeError> {
    let DbResponse::Many(result) =
        request(target, sql, parameters, Operation::TransactionMany(id))?
    else {
        unreachable!("database worker returned the wrong transaction read result")
    };
    Ok(result)
}

pub(super) fn finish_transaction(
    target: String,
    id: u64,
    commit: bool,
) -> Result<(), DbRuntimeError> {
    let DbResponse::TransactionFinished = request(
        target,
        String::new(),
        Vec::new(),
        Operation::TransactionFinish { id, commit },
    )?
    else {
        unreachable!("database worker returned the wrong transaction completion result")
    };
    Ok(())
}

pub(super) fn query_one(
    path: String,
    sql: String,
    parameters: Vec<DbValue>,
) -> Result<Option<DbRow>, DbRuntimeError> {
    let DbResponse::One(row) = request(path, sql, parameters, Operation::One)? else {
        unreachable!("database worker returned the wrong operation")
    };
    Ok(row)
}

pub(super) fn query_many(
    path: String,
    sql: String,
    parameters: Vec<DbValue>,
) -> Result<Vec<DbRow>, DbRuntimeError> {
    let DbResponse::Many(rows) = request(path, sql, parameters, Operation::Many)? else {
        unreachable!("database worker returned the wrong operation")
    };
    Ok(rows)
}

pub(super) fn query_page(
    path: String,
    rows_sql: String,
    rows_parameters: Vec<DbValue>,
    count_sql: String,
    count_parameters: Vec<DbValue>,
    page_size: u64,
    page_number: u64,
) -> Result<(Vec<DbRow>, u64), DbRuntimeError> {
    if page_size == 0 || page_number == 0 {
        return Err(invalid_pagination());
    }
    let offset = page_number
        .checked_sub(1)
        .and_then(|page| page.checked_mul(page_size))
        .ok_or_else(invalid_pagination)?;
    let mut rows_parameters = rows_parameters;
    rows_parameters.push(DbValue::Unsigned(page_size));
    rows_parameters.push(DbValue::Unsigned(offset));
    let DbResponse::Page { rows, total } = request(
        path,
        format!("{rows_sql} LIMIT ? OFFSET ?"),
        rows_parameters,
        Operation::Page(DbStatement::new(
            count_sql.into_boxed_str(),
            count_parameters,
        )),
    )?
    else {
        unreachable!("database worker returned the wrong operation")
    };
    Ok((rows, total))
}

pub(super) fn query_cursor_page(
    path: String,
    rows_sql: String,
    mut rows_parameters: Vec<DbValue>,
    count_sql: String,
    count_parameters: Vec<DbValue>,
    page_size: u64,
    cursor: Option<DbValue>,
) -> Result<(Vec<DbRow>, u64), DbRuntimeError> {
    if page_size == 0 {
        return Err(invalid_pagination());
    }
    let fetch_size = page_size.checked_add(1).ok_or_else(invalid_pagination)?;
    rows_parameters.push(DbValue::Bool(cursor.is_some()));
    rows_parameters.push(cursor.unwrap_or(DbValue::Null));
    rows_parameters.push(DbValue::Unsigned(fetch_size));
    let DbResponse::Page { rows, total } = request(
        path,
        format!("{rows_sql} LIMIT ?"),
        rows_parameters,
        Operation::Page(DbStatement::new(
            count_sql.into_boxed_str(),
            count_parameters,
        )),
    )?
    else {
        unreachable!("database worker returned the wrong operation")
    };
    Ok((rows, total))
}

pub(super) fn mongo_read(
    target: String,
    descriptor: String,
    parameters: Vec<DbValue>,
    operation: MongoReadOperation,
) -> Result<serde_json::Value, DbRuntimeError> {
    let DbResponse::Json(value) =
        request(target, descriptor, parameters, Operation::Mongo(operation))?
    else {
        unreachable!("database worker returned the wrong MongoDB operation")
    };
    Ok(value)
}

pub(super) fn mongo_mutate(
    target: String,
    descriptor: String,
    parameters: Vec<DbValue>,
) -> Result<serde_json::Value, DbRuntimeError> {
    let DbResponse::Json(result) =
        request(target, descriptor, parameters, Operation::MongoMutation)?
    else {
        unreachable!("database worker returned the wrong MongoDB mutation")
    };
    Ok(result)
}

fn invalid_pagination() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_INVALID_PAGINATION",
        message: "page and pageSize must be positive and their offset must fit in u64",
    }
}

fn request(
    path: String,
    sql: String,
    parameters: Vec<DbValue>,
    operation: Operation,
) -> Result<DbResponse, DbRuntimeError> {
    if SHUTTING_DOWN.load(Ordering::Acquire) {
        return Err(unavailable());
    }
    let service = SERVICE
        .get_or_init(|| {
            let (sender, receiver) = mpsc::sync_channel(DB_REQUEST_QUEUE_CAPACITY);
            let shutdown = Arc::new((Mutex::new(false), Condvar::new()));
            let worker_shutdown = shutdown.clone();
            std::thread::Builder::new()
                .name("tyrus-db".to_string())
                .spawn(move || worker(receiver, worker_shutdown))
                .map_err(|_| unavailable())?;
            Ok(Service { sender, shutdown })
        })
        .as_ref()
        .map_err(|error| *error)?;
    let (reply, answer) = mpsc::sync_channel(1);
    let request = Request {
        target: path,
        statement: DbStatement::new(sql.into_boxed_str(), parameters),
        operation,
        reply,
    };
    match service.sender.try_send(request) {
        Ok(()) => {}
        Err(mpsc::TrySendError::Full(_)) => return Err(database_busy()),
        Err(mpsc::TrySendError::Disconnected(_)) => return Err(unavailable()),
    }
    answer.recv().map_err(|_| unavailable())?
}

pub(super) fn shutdown() {
    SHUTTING_DOWN.store(true, Ordering::Release);
    let Some(Ok(service)) = SERVICE.get() else {
        return;
    };
    let (stopped, ready) = &*service.shutdown;
    let Ok(mut stopped) = stopped.lock() else {
        return;
    };
    let deadline = std::time::Instant::now() + Duration::from_millis(100);
    while !*stopped {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let Ok((next, timeout)) = ready.wait_timeout(stopped, remaining) else {
            return;
        };
        stopped = next;
        if timeout.timed_out() {
            break;
        }
    }
}

fn database_busy() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_BUSY",
        message: "database request queue is full; retry the operation later",
    }
}

fn worker(receiver: mpsc::Receiver<Request>, shutdown: Arc<(Mutex<bool>, Condvar)>) {
    let runtime = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build();
    let mut databases = HashMap::<String, SqliteDatabase>::new();
    let mut server_databases = HashMap::<String, ServerDatabase>::new();
    let mut mongo_databases = HashMap::<String, MongoDatabase>::new();
    let mut transactions = HashMap::<u64, TransactionEntry>::new();
    let permits = std::sync::Arc::new(tokio::sync::Semaphore::new(DB_CONCURRENT_REQUEST_LIMIT));
    let mut tasks = JoinSet::new();
    loop {
        if SHUTTING_DOWN.load(Ordering::Acquire) {
            break;
        }
        let request = match receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(request) => request,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        if SHUTTING_DOWN.load(Ordering::Acquire) {
            let _ = request.reply.send(Err(unavailable()));
            break;
        }
        let target = match parse_target(&request.target) {
            Ok(target) => target,
            Err(error) => {
                let _ = request.reply.send(Err(error));
                continue;
            }
        };
        let Ok(runtime) = &runtime else {
            let _ = request.reply.send(Err(unavailable()));
            continue;
        };
        let permit = match runtime.block_on(permits.clone().acquire_owned()) {
            Ok(permit) => permit,
            Err(_) => {
                let _ = request.reply.send(Err(unavailable()));
                continue;
            }
        };
        match target {
            RuntimeTarget::Sqlite(path, options) => {
                if !databases.contains_key(&request.target) {
                    match runtime.block_on(SqliteDatabase::open_with_options(&path, options)) {
                        Ok(database) => {
                            databases.insert(request.target.clone(), database);
                        }
                        Err(error) => {
                            let _ = request.reply.send(Err(error));
                            drop(permit);
                            continue;
                        }
                    }
                }
                let database = databases
                    .get(&request.target)
                    .expect("database was inserted before use")
                    .clone();
                if let Some(result) = handle_transaction_request(
                    &request.target,
                    &request,
                    runtime,
                    &mut transactions,
                    TransactionDatabase::Sqlite(&database),
                ) {
                    let _ = request.reply.send(result);
                    drop(permit);
                    continue;
                }
                {
                    let _entered = runtime.enter();
                    tasks.spawn(async move {
                        let result = run_sqlite_request(&database, &request).await;
                        let _ = request.reply.send(result);
                        drop(permit);
                    });
                }
            }
            RuntimeTarget::Server(backend, env_name, pool, ca_file, cert_file, key_file) => {
                if !server_databases.contains_key(&request.target) {
                    let Some(mut connection_string) = crate::env::lookup(&env_name) else {
                        let _ = request.reply.send(Err(missing_connection_variable()));
                        drop(permit);
                        continue;
                    };
                    let opened = runtime.block_on(ServerDatabase::open(
                        backend,
                        &connection_string,
                        pool,
                        ca_file.as_deref(),
                        cert_file.as_deref(),
                        key_file.as_deref(),
                    ));
                    use zeroize::Zeroize;
                    connection_string.zeroize();
                    match opened {
                        Ok(database) => {
                            server_databases.insert(request.target.clone(), database);
                        }
                        Err(error) => {
                            let _ = request.reply.send(Err(error));
                            drop(permit);
                            continue;
                        }
                    }
                }
                let database = server_databases
                    .get(&request.target)
                    .expect("server database was inserted before use")
                    .clone();
                if let Some(result) = handle_transaction_request(
                    &request.target,
                    &request,
                    runtime,
                    &mut transactions,
                    TransactionDatabase::Server(&database),
                ) {
                    let _ = request.reply.send(result);
                    drop(permit);
                    continue;
                }
                {
                    let _entered = runtime.enter();
                    tasks.spawn(async move {
                        let result = run_server_request(&database, &request).await;
                        let _ = request.reply.send(result);
                        drop(permit);
                    });
                }
            }
            RuntimeTarget::Mongodb {
                connection_env,
                pool,
                ca_file,
                cert_file,
                key_file,
            } => {
                if matches!(
                    &request.operation,
                    Operation::TransactionStart
                        | Operation::TransactionExec(_)
                        | Operation::TransactionOne(_)
                        | Operation::TransactionMany(_)
                        | Operation::TransactionFinish { .. }
                ) {
                    let _ = request.reply.send(Err(unsupported_transaction()));
                    drop(permit);
                    continue;
                }
                if !mongo_databases.contains_key(&request.target) {
                    let Some(mut connection_string) = crate::env::lookup(&connection_env) else {
                        let _ = request.reply.send(Err(missing_connection_variable()));
                        drop(permit);
                        continue;
                    };
                    let opened = runtime.block_on(MongoDatabase::open(
                        &connection_string,
                        pool,
                        ca_file.as_deref(),
                        cert_file.as_deref(),
                        key_file.as_deref(),
                    ));
                    use zeroize::Zeroize;
                    connection_string.zeroize();
                    match opened {
                        Ok(database) => {
                            mongo_databases.insert(request.target.clone(), database);
                        }
                        Err(error) => {
                            let _ = request.reply.send(Err(mongo_error(error)));
                            drop(permit);
                            continue;
                        }
                    }
                }
                let database = mongo_databases
                    .get(&request.target)
                    .expect("MongoDB client was inserted before use")
                    .clone();
                if let Some(result) = handle_transaction_request(
                    &request.target,
                    &request,
                    runtime,
                    &mut transactions,
                    TransactionDatabase::Unsupported,
                ) {
                    let _ = request.reply.send(result);
                    drop(permit);
                    continue;
                }
                let operation = request.operation;
                let (descriptor, parameters) = request.statement.into_parts();
                let descriptor = descriptor.into_string();
                {
                    let _entered = runtime.enter();
                    tasks.spawn(async move {
                        let result = match operation {
                            Operation::Mongo(operation) => database
                                .read(operation, &descriptor, &parameters)
                                .await
                                .map(DbResponse::Json)
                                .map_err(mongo_error),
                            Operation::MongoMutation => database
                                .mutate(&descriptor, &parameters)
                                .await
                                .map(DbResponse::Json)
                                .map_err(mongo_error),
                            _ => Err(mongo_error(database.unsupported_operation())),
                        };
                        let _ = request.reply.send(result);
                        drop(permit);
                    });
                }
            }
        }
    }
    if let Ok(runtime) = runtime {
        let _ = runtime.block_on(drain_and_cancel_tasks(
            &mut tasks,
            Duration::from_millis(100),
        ));
        let close = async {
            transactions.clear();
            for database in databases.into_values() {
                database.close().await;
            }
            for database in server_databases.into_values() {
                database.close().await;
            }
            for database in mongo_databases.into_values() {
                database.close().await;
            }
        };
        let _ = runtime
            .block_on(async { tokio::time::timeout(Duration::from_millis(100), close).await });
        runtime.shutdown_timeout(Duration::from_millis(100));
    }
    let (stopped, ready) = &*shutdown;
    if let Ok(mut stopped) = stopped.lock() {
        *stopped = true;
        ready.notify_all();
    }
}

async fn drain_and_cancel_tasks(tasks: &mut JoinSet<()>, timeout: Duration) -> bool {
    let drain = async { while tasks.join_next().await.is_some() {} };
    if tokio::time::timeout(timeout, drain).await.is_ok() {
        return true;
    }

    tasks.abort_all();
    let cancellation = async { while tasks.join_next().await.is_some() {} };
    let _ = tokio::time::timeout(timeout, cancellation).await;
    false
}

fn handle_transaction_request(
    target: &str,
    request: &Request,
    runtime: &tokio::runtime::Runtime,
    transactions: &mut HashMap<u64, TransactionEntry>,
    database: TransactionDatabase<'_>,
) -> Option<Result<DbResponse, DbRuntimeError>> {
    match &request.operation {
        Operation::TransactionStart => Some(match database {
            TransactionDatabase::Sqlite(database) => {
                runtime.block_on(database.begin()).and_then(|transaction| {
                    let id = next_transaction_id()?;
                    transactions.insert(
                        id,
                        TransactionEntry {
                            target: target.to_string(),
                            transaction: ActiveTransaction::Sqlite(transaction),
                        },
                    );
                    Ok(DbResponse::TransactionId(id))
                })
            }
            TransactionDatabase::Server(database) => runtime
                .block_on(database.begin_transaction())
                .and_then(|transaction| {
                    let id = next_transaction_id()?;
                    transactions.insert(
                        id,
                        TransactionEntry {
                            target: target.to_string(),
                            transaction: ActiveTransaction::Server(transaction),
                        },
                    );
                    Ok(DbResponse::TransactionId(id))
                }),
            TransactionDatabase::Unsupported => Err(unsupported_transaction()),
        }),
        Operation::TransactionExec(id) => Some(rollback_failed_operation(
            (|| {
                let entry = transactions
                    .get_mut(id)
                    .filter(|entry| entry.target == target)
                    .ok_or_else(invalid_transaction)?;
                match (&mut entry.transaction, database) {
                    (
                        ActiveTransaction::Sqlite(transaction),
                        TransactionDatabase::Sqlite(database),
                    ) => runtime
                        .block_on(database.execute_in(transaction, &request.statement))
                        .map(DbResponse::Exec),
                    (
                        ActiveTransaction::Server(transaction),
                        TransactionDatabase::Server(database),
                    ) => runtime
                        .block_on(database.execute_in_transaction(transaction, &request.statement))
                        .map(DbResponse::Exec),
                    _ => Err(invalid_transaction()),
                }
            })(),
            target,
            *id,
            runtime,
            transactions,
        )),
        Operation::TransactionMutationReadback {
            id,
            query,
            read_before,
            use_inserted_id,
        } => Some(rollback_failed_operation(
            (|| {
                let entry = transactions
                    .get_mut(id)
                    .filter(|entry| entry.target == target)
                    .ok_or_else(invalid_transaction)?;
                match (&mut entry.transaction, database) {
                    (
                        ActiveTransaction::Sqlite(transaction),
                        TransactionDatabase::Sqlite(database),
                    ) => runtime
                        .block_on(sqlite_mutate_readback(
                            database,
                            transaction,
                            &request.statement,
                            query,
                            *read_before,
                            *use_inserted_id,
                        ))
                        .map(DbResponse::One),
                    (
                        ActiveTransaction::Server(transaction),
                        TransactionDatabase::Server(database),
                    ) => runtime
                        .block_on(server_mutate_readback(
                            database,
                            transaction,
                            &request.statement,
                            query,
                            *read_before,
                            *use_inserted_id,
                        ))
                        .map(DbResponse::One),
                    _ => Err(invalid_transaction()),
                }
            })(),
            target,
            *id,
            runtime,
            transactions,
        )),
        Operation::TransactionOne(id) => Some(rollback_failed_operation(
            (|| {
                let entry = transactions
                    .get_mut(id)
                    .filter(|entry| entry.target == target)
                    .ok_or_else(invalid_transaction)?;
                match (&mut entry.transaction, database) {
                    (
                        ActiveTransaction::Sqlite(transaction),
                        TransactionDatabase::Sqlite(database),
                    ) => runtime
                        .block_on(database.query_one_in(transaction, &request.statement))
                        .map(DbResponse::One),
                    (
                        ActiveTransaction::Server(transaction),
                        TransactionDatabase::Server(database),
                    ) => runtime
                        .block_on(
                            database.query_one_in_transaction(transaction, &request.statement),
                        )
                        .map(DbResponse::One),
                    _ => Err(invalid_transaction()),
                }
            })(),
            target,
            *id,
            runtime,
            transactions,
        )),
        Operation::TransactionMany(id) => Some(rollback_failed_operation(
            (|| {
                let entry = transactions
                    .get_mut(id)
                    .filter(|entry| entry.target == target)
                    .ok_or_else(invalid_transaction)?;
                match (&mut entry.transaction, database) {
                    (
                        ActiveTransaction::Sqlite(transaction),
                        TransactionDatabase::Sqlite(database),
                    ) => runtime
                        .block_on(database.query_many_in(transaction, &request.statement))
                        .map(DbResponse::Many),
                    (
                        ActiveTransaction::Server(transaction),
                        TransactionDatabase::Server(database),
                    ) => runtime
                        .block_on(
                            database.query_many_in_transaction(transaction, &request.statement),
                        )
                        .map(DbResponse::Many),
                    _ => Err(invalid_transaction()),
                }
            })(),
            target,
            *id,
            runtime,
            transactions,
        )),
        Operation::TransactionFinish { id, commit } => Some((|| {
            if !transactions
                .get(id)
                .is_some_and(|entry| entry.target == target)
            {
                return Err(invalid_transaction());
            }
            let entry = transactions.remove(id).ok_or_else(invalid_transaction)?;
            let result = match entry.transaction {
                ActiveTransaction::Sqlite(transaction) => {
                    if *commit {
                        runtime.block_on(SqliteDatabase::commit(transaction))
                    } else {
                        runtime.block_on(SqliteDatabase::rollback(transaction))
                    }
                }
                ActiveTransaction::Server(transaction) => {
                    if *commit {
                        runtime.block_on(ServerDatabase::commit_transaction(transaction))
                    } else {
                        runtime.block_on(ServerDatabase::rollback_transaction(transaction))
                    }
                }
            };
            result.map(|()| DbResponse::TransactionFinished)
        })()),
        _ => None,
    }
}

fn rollback_failed_operation(
    result: Result<DbResponse, DbRuntimeError>,
    target: &str,
    id: u64,
    runtime: &tokio::runtime::Runtime,
    transactions: &mut HashMap<u64, TransactionEntry>,
) -> Result<DbResponse, DbRuntimeError> {
    if result.is_err() {
        if !transactions
            .get(&id)
            .is_some_and(|entry| entry.target == target)
        {
            return result;
        }
        if let Some(entry) = transactions.remove(&id) {
            match entry.transaction {
                ActiveTransaction::Sqlite(transaction) => {
                    let _ = runtime.block_on(SqliteDatabase::rollback(transaction));
                }
                ActiveTransaction::Server(transaction) => {
                    let _ = runtime.block_on(ServerDatabase::rollback_transaction(transaction));
                }
            }
        }
    }
    result
}

fn next_transaction_id() -> Result<u64, DbRuntimeError> {
    NEXT_TRANSACTION_ID
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(1)
        })
        .map_err(|_| transaction_limit_reached())
}

fn transaction_limit_reached() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_TRANSACTION_LIMIT",
        message: "database transaction handle capacity has been exhausted",
    }
}

fn invalid_transaction() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_INVALID_TRANSACTION",
        message: "transaction handle is invalid, closed, or belongs to another database",
    }
}

fn unsupported_transaction() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_TRANSACTION_UNSUPPORTED",
        message: "transactions are not available for this database backend",
    }
}

async fn run_sqlite_request(
    database: &SqliteDatabase,
    request: &Request,
) -> Result<DbResponse, DbRuntimeError> {
    match &request.operation {
        Operation::Exec => database
            .execute(&request.statement)
            .await
            .map(DbResponse::Exec),
        Operation::MutationReadback {
            query,
            read_before,
            use_inserted_id,
        } => sqlite_mutate_readback_owned(
            database,
            &request.statement,
            query,
            *read_before,
            *use_inserted_id,
        )
        .await
        .map(DbResponse::One),
        Operation::One => database
            .query_one(&request.statement)
            .await
            .map(DbResponse::One),
        Operation::Many => database
            .query_many(&request.statement)
            .await
            .map(DbResponse::Many),
        Operation::Page(count_statement) => {
            let mut transaction = database.begin().await?;

            async {
                let count_row = database
                    .query_one_in(&mut transaction, count_statement)
                    .await?
                    .ok_or_else(DbRuntimeError::decode)?;
                let total = match count_row.values().first() {
                    Some(DbValue::Signed(value)) => {
                        u64::try_from(*value).map_err(|_| DbRuntimeError::decode())?
                    }
                    Some(DbValue::Unsigned(value)) => *value,
                    _ => return Err(DbRuntimeError::decode()),
                };
                let rows = database
                    .query_many_in(&mut transaction, &request.statement)
                    .await?;
                SqliteDatabase::commit(transaction).await?;
                Ok(DbResponse::Page { rows, total })
            }
            .await
        }
        Operation::Transaction(statements) => database
            .execute_transaction(statements)
            .await
            .map(DbResponse::Transaction),
        Operation::TransactionStart
        | Operation::TransactionExec(_)
        | Operation::TransactionMutationReadback { .. }
        | Operation::TransactionOne(_)
        | Operation::TransactionMany(_)
        | Operation::TransactionFinish { .. } => Err(invalid_transaction()),
        Operation::Mongo(_) | Operation::MongoMutation => Err(invalid_target()),
    }
}

async fn run_server_request(
    database: &ServerDatabase,
    request: &Request,
) -> Result<DbResponse, DbRuntimeError> {
    match &request.operation {
        Operation::Exec => database
            .execute(&request.statement)
            .await
            .map(DbResponse::Exec),
        Operation::MutationReadback {
            query,
            read_before,
            use_inserted_id,
        } => server_mutate_readback_owned(
            database,
            &request.statement,
            query,
            *read_before,
            *use_inserted_id,
        )
        .await
        .map(DbResponse::One),
        Operation::One => database
            .query_one(&request.statement)
            .await
            .map(DbResponse::One),
        Operation::Many => database
            .query_many(&request.statement)
            .await
            .map(DbResponse::Many),
        Operation::Page(count_statement) => database
            .query_page(&request.statement, count_statement)
            .await
            .map(|(rows, total)| DbResponse::Page { rows, total }),
        Operation::Transaction(statements) => database
            .execute_transaction(statements)
            .await
            .map(DbResponse::Transaction),
        Operation::TransactionStart
        | Operation::TransactionExec(_)
        | Operation::TransactionMutationReadback { .. }
        | Operation::TransactionOne(_)
        | Operation::TransactionMany(_)
        | Operation::TransactionFinish { .. } => Err(invalid_transaction()),
        Operation::Mongo(_) | Operation::MongoMutation => Err(invalid_target()),
    }
}

async fn sqlite_mutate_readback_owned(
    database: &SqliteDatabase,
    mutation: &DbStatement,
    query: &DbStatement,
    read_before: bool,
    use_inserted_id: bool,
) -> Result<Option<DbRow>, DbRuntimeError> {
    let mut transaction = database.begin().await?;
    let result = sqlite_mutate_readback(
        database,
        &mut transaction,
        mutation,
        query,
        read_before,
        use_inserted_id,
    )
    .await;
    match result {
        Ok(row) => {
            SqliteDatabase::commit(transaction).await?;
            Ok(row)
        }
        Err(error) => {
            let _ = SqliteDatabase::rollback(transaction).await;
            Err(error)
        }
    }
}

async fn sqlite_mutate_readback(
    database: &SqliteDatabase,
    transaction: &mut Transaction<'static, Sqlite>,
    mutation: &DbStatement,
    query: &DbStatement,
    read_before: bool,
    use_inserted_id: bool,
) -> Result<Option<DbRow>, DbRuntimeError> {
    let before = if read_before {
        database.query_one_in(transaction, query).await?
    } else {
        None
    };
    let execution = database.execute_in(transaction, mutation).await?;
    if read_before {
        return Ok(before);
    }
    if use_inserted_id {
        let (sql, mut parameters) = query.clone().into_parts();
        parameters.push(DbValue::Signed(execution.last_insert_rowid));
        let readback = DbStatement::new(sql, parameters);
        database.query_one_in(transaction, &readback).await
    } else {
        database.query_one_in(transaction, query).await
    }
}

async fn server_mutate_readback_owned(
    database: &ServerDatabase,
    mutation: &DbStatement,
    query: &DbStatement,
    read_before: bool,
    use_inserted_id: bool,
) -> Result<Option<DbRow>, DbRuntimeError> {
    let mut transaction = database.begin_transaction().await?;
    let result = server_mutate_readback(
        database,
        &mut transaction,
        mutation,
        query,
        read_before,
        use_inserted_id,
    )
    .await;
    match result {
        Ok(row) => {
            ServerDatabase::commit_transaction(transaction).await?;
            Ok(row)
        }
        Err(error) => {
            let _ = ServerDatabase::rollback_transaction(transaction).await;
            Err(error)
        }
    }
}

async fn server_mutate_readback(
    database: &ServerDatabase,
    transaction: &mut Transaction<'static, Any>,
    mutation: &DbStatement,
    query: &DbStatement,
    read_before: bool,
    use_inserted_id: bool,
) -> Result<Option<DbRow>, DbRuntimeError> {
    let before = if read_before {
        database
            .query_one_in_transaction(transaction, query)
            .await?
    } else {
        None
    };
    let execution = database
        .execute_in_transaction(transaction, mutation)
        .await?;
    if read_before {
        return Ok(before);
    }
    if use_inserted_id {
        let (sql, mut parameters) = query.clone().into_parts();
        parameters.push(DbValue::Signed(execution.last_insert_rowid));
        let readback = DbStatement::new(sql, parameters);
        database
            .query_one_in_transaction(transaction, &readback)
            .await
    } else {
        database.query_one_in_transaction(transaction, query).await
    }
}

enum RuntimeTarget {
    Sqlite(String, SqliteOptions),
    Server(
        ServerBackend,
        String,
        ServerPoolOptions,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
    Mongodb {
        connection_env: String,
        pool: MongoPoolOptions,
        ca_file: Option<String>,
        cert_file: Option<String>,
        key_file: Option<String>,
    },
}

#[derive(Default)]
struct DbTlsConfig {
    ca_file: Option<String>,
    cert_file: Option<String>,
    key_file: Option<String>,
}

fn parse_target(target: &str) -> Result<RuntimeTarget, DbRuntimeError> {
    if !target.starts_with('{') {
        return Ok(RuntimeTarget::Sqlite(
            target.to_string(),
            SqliteOptions::default(),
        ));
    }
    let value: serde_json::Value = serde_json::from_str(target).map_err(|_| invalid_target())?;
    let backend = value
        .get("backend")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(invalid_target)?;
    match backend {
        "sqlite" => {
            let path = value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(invalid_target)?;
            let options =
                SqliteOptions::from_json(value.get("sqlite")).ok_or_else(invalid_target)?;
            Ok(RuntimeTarget::Sqlite(path.to_string(), options))
        }
        "postgresql" | "mysql" => {
            let connection_env = value
                .get("connectionEnv")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(invalid_target)?
                .to_string();
            let backend = if backend == "postgresql" {
                ServerBackend::Postgresql
            } else {
                ServerBackend::Mysql
            };
            let pool = match value.get("pool") {
                Some(pool) => ServerPoolOptions::from_json(pool).ok_or_else(invalid_target)?,
                None => ServerPoolOptions::default(),
            };
            if !pool.validate() {
                return Err(invalid_target());
            }
            let tls = parse_tls_target(value.get("tls"))?;
            Ok(RuntimeTarget::Server(
                backend,
                connection_env,
                pool,
                tls.ca_file,
                tls.cert_file,
                tls.key_file,
            ))
        }
        "mongodb" => {
            let connection_env = value
                .get("connectionEnv")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(invalid_target)?
                .to_string();
            let pool = match value.get("pool") {
                Some(pool) => MongoPoolOptions::from_json(pool).ok_or_else(invalid_target)?,
                None => MongoPoolOptions::default(),
            };
            if !pool.validate() {
                return Err(invalid_target());
            }
            let tls = parse_tls_target(value.get("tls"))?;
            Ok(RuntimeTarget::Mongodb {
                connection_env,
                pool,
                ca_file: tls.ca_file,
                cert_file: tls.cert_file,
                key_file: tls.key_file,
            })
        }
        _ => Err(invalid_target()),
    }
}

fn parse_tls_target(value: Option<&serde_json::Value>) -> Result<DbTlsConfig, DbRuntimeError> {
    let Some(value) = value else {
        return Ok(DbTlsConfig::default());
    };
    let tls = value.as_object().ok_or_else(invalid_target)?;
    if tls
        .keys()
        .any(|key| !matches!(key.as_str(), "caFile" | "certFile" | "keyFile"))
    {
        return Err(invalid_target());
    }
    let path = |key: &str| -> Result<Option<String>, DbRuntimeError> {
        match tls.get(key) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(value) => value
                .as_str()
                .map(|value| Some(value.to_string()))
                .ok_or_else(invalid_target),
        }
    };
    let ca_file = path("caFile")?;
    let cert_file = path("certFile")?;
    let key_file = path("keyFile")?;
    if cert_file.is_some() != key_file.is_some() {
        return Err(invalid_target());
    }
    Ok(DbTlsConfig {
        ca_file,
        cert_file,
        key_file,
    })
}

fn mongo_error(error: MongoRuntimeError) -> DbRuntimeError {
    DbRuntimeError {
        code: error.code,
        message: error.message,
    }
}

fn missing_connection_variable() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_CONNECTION_CONFIGURATION",
        message: "the configured database connection environment variable is missing",
    }
}

fn invalid_target() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_INVALID_CONFIGURATION",
        message: "database runtime target is invalid",
    }
}

fn unavailable() -> DbRuntimeError {
    DbRuntimeError {
        code: "ERR_DB_RUNTIME_UNAVAILABLE",
        message: "database runtime is unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_database_service() -> std::sync::MutexGuard<'static, ()> {
        crate::db::TEST_SERVICE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn database_runtime_targets_require_backend_specific_configuration() {
        for target in [
            "{",
            r#"{"backend":"unsupported"}"#,
            r#"{"backend":"sqlite"}"#,
            r#"{"backend":"postgresql","urlEnv":"DATABASE_URL"}"#,
            r#"{"backend":"mysql","connectionEnv":"DATABASE_URL","pool":{"maxConnections":0}}"#,
            r#"{"backend":"mongodb","connectionEnv":"DATABASE_URL","pool":{"maxLifetimeMs":1000}}"#,
            r#"{"backend":"postgresql","connectionEnv":"DATABASE_URL","tls":{"certFile":"client.pem"}}"#,
            r#"{"backend":"mongodb","connectionEnv":"DATABASE_URL","tls":{"verify":false}}"#,
        ] {
            let result = parse_target(target);
            assert!(
                result.is_err(),
                "accepted invalid database target: {target}"
            );
            assert_eq!(
                result.err().map(|error| error.code),
                Some("ERR_DB_INVALID_CONFIGURATION")
            );
        }

        assert!(matches!(
            parse_target(r#"{"backend":"postgresql","connectionEnv":"DATABASE_URL"}"#),
            Ok(RuntimeTarget::Server(ServerBackend::Postgresql, ..))
        ));
        assert!(matches!(
            parse_target(r#"{"backend":"mysql","connectionEnv":"DATABASE_URL"}"#),
            Ok(RuntimeTarget::Server(ServerBackend::Mysql, ..))
        ));
        assert!(matches!(
            parse_target(r#"{"backend":"mongodb","connectionEnv":"DATABASE_URL"}"#),
            Ok(RuntimeTarget::Mongodb { .. })
        ));
    }

    #[test]
    fn database_service_redacts_missing_variables_and_rejects_mongo_transactions() {
        let _guard = lock_database_service();
        let suffix = format!(
            "{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        for backend in ["postgresql", "mysql", "mongodb"] {
            let connection_env = format!("TYRUS_INTERNAL_MISSING_DB_TEST_{suffix}_{backend}");
            assert!(std::env::var_os(&connection_env).is_none());
            let target = serde_json::json!({
                "backend": backend,
                "connectionEnv": connection_env
            })
            .to_string();
            let result = query_one(target, String::new(), Vec::new());
            match result {
                Err(error) => {
                    assert_eq!(error.code, "ERR_DB_CONNECTION_CONFIGURATION");
                    assert!(!error.message.contains(&connection_env));
                }
                Ok(_) => panic!("missing connection variable unexpectedly opened a database"),
            }
        }

        let connection_env = format!("TYRUS_INTERNAL_MONGO_TRANSACTION_TEST_{suffix}");
        assert!(std::env::var_os(&connection_env).is_none());
        let target = serde_json::json!({
            "backend": "mongodb",
            "connectionEnv": connection_env
        })
        .to_string();
        let error = begin_transaction(target).unwrap_err();
        assert_eq!(error.code, "ERR_DB_TRANSACTION_UNSUPPORTED");
    }

    #[test]
    fn one_many_and_exec_share_the_same_memory_database() {
        let _guard = lock_database_service();
        let path = ":memory:".to_string();
        execute(
            path.clone(),
            "CREATE TABLE service_items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)".to_string(),
            Vec::new(),
        )
        .unwrap();
        let inserted = execute(
            path.clone(),
            "INSERT INTO service_items (name) VALUES (?)".to_string(),
            vec![DbValue::Text("alpha".to_string())],
        )
        .unwrap();
        assert_eq!(inserted.rows_affected, 1);
        let found = query_one(
            path.clone(),
            "SELECT name FROM service_items WHERE id = ?".to_string(),
            vec![DbValue::Signed(inserted.last_insert_rowid)],
        )
        .unwrap()
        .unwrap();
        assert_eq!(found.values(), &[DbValue::Text("alpha".to_string())]);
        let rows = query_many(
            path,
            "SELECT name FROM service_items ORDER BY id".to_string(),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].values(), found.values());
    }

    #[test]
    fn transaction_batch_is_atomic_through_the_database_service() {
        let _guard = lock_database_service();
        let target = ":memory:".to_string();
        execute(
            target.clone(),
            "CREATE TABLE transaction_items (id INTEGER PRIMARY KEY)".to_string(),
            Vec::new(),
        )
        .unwrap();

        let committed = execute_transaction(
            target.clone(),
            vec![
                DbStatement::new(
                    "INSERT INTO transaction_items VALUES (?)".into(),
                    vec![DbValue::Signed(1)],
                ),
                DbStatement::new(
                    "INSERT INTO transaction_items VALUES (?)".into(),
                    vec![DbValue::Signed(2)],
                ),
            ],
        )
        .unwrap();
        assert_eq!(committed.len(), 2);

        let failed = execute_transaction(
            target.clone(),
            vec![
                DbStatement::new(
                    "INSERT INTO transaction_items VALUES (?)".into(),
                    vec![DbValue::Signed(3)],
                ),
                DbStatement::new(
                    "INSERT INTO transaction_items VALUES (?)".into(),
                    vec![DbValue::Signed(1)],
                ),
            ],
        )
        .unwrap_err();
        assert_eq!(failed.code, "ERR_DB_SQLITE_CONSTRAINT");

        let rows = query_many(
            target,
            "SELECT id FROM transaction_items ORDER BY id".to_string(),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].values(), &[DbValue::Signed(1)]);
        assert_eq!(rows[1].values(), &[DbValue::Signed(2)]);
    }

    #[test]
    fn transaction_handle_keeps_reads_and_writes_on_the_same_connection() {
        let _guard = lock_database_service();
        let target = ":memory:".to_string();
        let table = format!("transaction_handle_{}", std::process::id());
        execute(
            target.clone(),
            format!("CREATE TABLE {table} (value INTEGER)"),
            Vec::new(),
        )
        .unwrap();

        let committed = begin_transaction(target.clone()).unwrap();
        execute_in_transaction(
            target.clone(),
            committed,
            format!("INSERT INTO {table} VALUES (?)"),
            vec![DbValue::Signed(10)],
        )
        .unwrap();
        let inside = query_one_in_transaction(
            target.clone(),
            committed,
            format!("SELECT value FROM {table}"),
            Vec::new(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(inside.values(), &[DbValue::Signed(10)]);
        let inside_many = query_many_in_transaction(
            target.clone(),
            committed,
            format!("SELECT value FROM {table}"),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(inside_many.len(), 1);
        finish_transaction(target.clone(), committed, true).unwrap();

        let rolled_back = begin_transaction(target.clone()).unwrap();
        execute_in_transaction(
            target.clone(),
            rolled_back,
            format!("INSERT INTO {table} VALUES (?)"),
            vec![DbValue::Signed(20)],
        )
        .unwrap();
        finish_transaction(target.clone(), rolled_back, false).unwrap();

        let rows = query_many(target, format!("SELECT value FROM {table}"), Vec::new()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].values(), &[DbValue::Signed(10)]);
    }

    #[test]
    fn failed_transaction_operation_rolls_back_and_invalidates_handle() {
        let _guard = lock_database_service();
        let target = ":memory:".to_string();
        let table = format!("transaction_failure_{}", std::process::id());
        execute(
            target.clone(),
            format!("CREATE TABLE {table} (value INTEGER NOT NULL)"),
            Vec::new(),
        )
        .unwrap();
        execute(
            target.clone(),
            format!("INSERT INTO {table} VALUES (1)"),
            Vec::new(),
        )
        .unwrap();

        let transaction = begin_transaction(target.clone()).unwrap();
        execute_in_transaction(
            target.clone(),
            transaction,
            format!("UPDATE {table} SET value = 2"),
            Vec::new(),
        )
        .unwrap();
        let failed = request(
            target.clone(),
            format!("UPDATE {table} SET missing_column = ?"),
            vec![DbValue::Signed(3)],
            Operation::TransactionExec(transaction),
        )
        .err()
        .unwrap();
        assert_eq!(failed.code, "ERR_DB_EXECUTE_FAILED");

        let reused = request(
            target.clone(),
            format!("SELECT value FROM {table}"),
            Vec::new(),
            Operation::TransactionMany(transaction),
        )
        .err()
        .unwrap();
        assert_eq!(reused.code, "ERR_DB_INVALID_TRANSACTION");

        let row = query_one(target, format!("SELECT value FROM {table}"), Vec::new())
            .unwrap()
            .unwrap();
        assert_eq!(row.values(), &[DbValue::Signed(1)]);
    }

    #[test]
    fn shutdown_drain_is_bounded_and_cancels_remaining_tasks() {
        struct DropFlag(Arc<AtomicBool>);

        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }

        let runtime = Builder::new_current_thread().enable_time().build().unwrap();
        runtime.block_on(async {
            let mut completed = JoinSet::new();
            completed.spawn(async {});
            assert!(drain_and_cancel_tasks(&mut completed, Duration::from_millis(10)).await);
            assert!(completed.is_empty());

            let dropped = Arc::new(AtomicBool::new(false));
            let mut pending = JoinSet::new();
            let task_dropped = dropped.clone();
            pending.spawn(async move {
                let _drop_flag = DropFlag(task_dropped);
                std::future::pending::<()>().await;
            });
            tokio::task::yield_now().await;
            assert!(!drain_and_cancel_tasks(&mut pending, Duration::from_millis(10)).await);
            assert!(pending.is_empty());
            assert!(dropped.load(Ordering::Acquire));
        });
    }

    #[test]
    fn shutdown_cancels_an_in_flight_database_request_within_bound() {
        const CHILD_ENV: &str = "TYRUS_INTERNAL_DB_SHUTDOWN_TEST_CHILD";
        if std::env::var(CHILD_ENV).as_deref() == Ok("1") {
            let target = ":memory:".to_string();
            execute(
                target.clone(),
                "CREATE TABLE shutdown_items (value INTEGER NOT NULL)".to_string(),
                Vec::new(),
            )
            .unwrap();
            let (reply, result) = mpsc::sync_channel(1);
            let query_target = target.clone();
            let query = std::thread::spawn(move || {
                let result = query_many(
                    query_target,
                    "WITH RECURSIVE sequence(value) AS (VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 100000000) SELECT value FROM sequence ORDER BY value DESC LIMIT 1".to_string(),
                    Vec::new(),
                );
                let _ = reply.send(result);
            });
            std::thread::sleep(Duration::from_millis(20));
            let started = std::time::Instant::now();
            shutdown();
            assert!(started.elapsed() < Duration::from_millis(500));
            let result = result
                .recv_timeout(Duration::from_millis(500))
                .expect("shutdown left the request caller blocked");
            match result {
                Err(error) => assert_eq!(error.code, "ERR_DB_RUNTIME_UNAVAILABLE"),
                Ok(_) => panic!("in-flight database request unexpectedly completed"),
            }
            query.join().unwrap();
            return;
        }

        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("db::service::tests::shutdown_cancels_an_in_flight_database_request_within_bound")
            .arg("--nocapture")
            .env(CHILD_ENV, "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let output = child.wait_with_output().unwrap();
                panic!(
                    "database shutdown child exceeded its deadline:\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "database shutdown child failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
