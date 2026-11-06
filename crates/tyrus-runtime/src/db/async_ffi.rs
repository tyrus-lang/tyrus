use super::ffi::{decode_parameters, encode_result, row_json};
use super::mongodb::MongoReadOperation;
use super::service;
use super::sqlite::{DbExecResult, DbRuntimeError};
use super::{DbRow, DbStatement, DbValue};
use crate::TyrusPromise;
use serde_json::{Map, Value};
use std::ffi::c_void;
use tyrus_string::TyrusString;

const INVALID_OPERATION: DbRuntimeError = DbRuntimeError {
    code: "ERR_DB_INVALID_OPERATION",
    message: "database operation code is not supported",
};
const PAYLOAD_TOO_LARGE: DbRuntimeError = DbRuntimeError {
    code: "ERR_DB_RESULT_TOO_LARGE",
    message: "database result exceeds the supported string size",
};
const INVALID_CURSOR: DbRuntimeError = DbRuntimeError {
    code: "ERR_DB_INVALID_CURSOR",
    message: "cursor token is malformed or does not match the model id type",
};
const INVALID_TRANSACTION_BATCH: DbRuntimeError = DbRuntimeError {
    code: "ERR_DB_INVALID_TRANSACTION",
    message: "transaction statements must be a non-empty JSON array of SQL statements and parameter arrays",
};
const TOO_MANY_TRANSACTION_PARAMETERS: DbRuntimeError = DbRuntimeError {
    code: "ERR_DB_PARAMETER_LIMIT",
    message: "a transaction cannot bind more than 999 parameters across its statements",
};

#[derive(Clone, Copy)]
enum AsyncDbOperation {
    Exec,
    One,
    Many,
    Count,
    Page,
    CursorPage,
    MongoCount,
    MongoOne,
    MongoMany,
    MongoMutation,
    MongoPage,
    MongoCursorPage,
    TransactionBatch,
    TransactionBegin,
    TransactionExec,
    TransactionOne,
    TransactionMany,
    TransactionCommit,
    TransactionRollback,
    TransactionCount,
    ExecCount,
    TransactionExecCount,
    MutationReadback,
    InsertManyReadback,
}

struct InsertManyReadback {
    mutations: Vec<DbStatement>,
    readbacks: Vec<DbStatement>,
    use_inserted_ids: Vec<bool>,
}

struct AsyncDbInput {
    operation: AsyncDbOperation,
    path: String,
    sql: String,
    parameters: String,
    count_sql: String,
    count_parameters: String,
    after: String,
    cursor_field: String,
    page_size: u64,
    page_number: u64,
    transaction_id: u64,
}

/// Schedules a database operation and fulfills the returned Promise with a JSON envelope.
///
/// # Safety
///
/// String arguments must be valid borrowed Tyrus strings. The function copies
/// their contents before returning and retains the Promise until the worker settles it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_db_async_query(
    operation: u32,
    path: *const TyrusString,
    sql: *const TyrusString,
    parameters: *const TyrusString,
    count_sql: *const TyrusString,
    count_parameters: *const TyrusString,
    after: *const TyrusString,
    cursor_field: *const TyrusString,
    page_size: u64,
    page_number: u64,
    transaction_id: u64,
) -> *mut TyrusPromise {
    unsafe {
        let Some(operation) = operation_from_code(operation) else {
            let promise = crate::tyrus_promise_pending();
            fulfill_envelope(promise, Err(INVALID_OPERATION));
            return promise;
        };
        let input = AsyncDbInput {
            operation,
            path: { &*path }.as_str().to_owned(),
            sql: { &*sql }.as_str().to_owned(),
            parameters: { &*parameters }.as_str().to_owned(),
            count_sql: { &*count_sql }.as_str().to_owned(),
            count_parameters: { &*count_parameters }.as_str().to_owned(),
            after: { &*after }.as_str().to_owned(),
            cursor_field: { &*cursor_field }.as_str().to_owned(),
            page_size,
            page_number,
            transaction_id,
        };
        let context = Box::into_raw(Box::new(Some(input))).cast::<c_void>();
        {
            crate::executor::tyrus_spawn_blocking(
                Some(run_async_db_task),
                context,
                Some(drop_async_db_task),
            )
        }
    }
}

unsafe extern "C" fn run_async_db_task(context: *mut c_void, promise: *mut TyrusPromise) {
    let input = unsafe { &mut *context.cast::<Option<AsyncDbInput>>() };
    let Some(input) = input.take() else {
        fulfill_envelope(promise, Err(INVALID_OPERATION));
        return;
    };
    fulfill_envelope(promise, run_operation(input));
}

unsafe extern "C" fn drop_async_db_task(context: *mut c_void) {
    if !context.is_null() {
        drop(unsafe { Box::from_raw(context.cast::<Option<AsyncDbInput>>()) });
    }
}

fn operation_from_code(operation: u32) -> Option<AsyncDbOperation> {
    match operation {
        0 => Some(AsyncDbOperation::Exec),
        1 => Some(AsyncDbOperation::One),
        2 => Some(AsyncDbOperation::Many),
        3 => Some(AsyncDbOperation::Count),
        4 => Some(AsyncDbOperation::Page),
        5 => Some(AsyncDbOperation::CursorPage),
        6 => Some(AsyncDbOperation::MongoCount),
        7 => Some(AsyncDbOperation::MongoOne),
        8 => Some(AsyncDbOperation::MongoMany),
        9 => Some(AsyncDbOperation::MongoMutation),
        10 => Some(AsyncDbOperation::MongoPage),
        11 => Some(AsyncDbOperation::MongoCursorPage),
        12 => Some(AsyncDbOperation::TransactionBatch),
        13 => Some(AsyncDbOperation::TransactionBegin),
        14 => Some(AsyncDbOperation::TransactionExec),
        15 => Some(AsyncDbOperation::TransactionOne),
        16 => Some(AsyncDbOperation::TransactionMany),
        17 => Some(AsyncDbOperation::TransactionCommit),
        18 => Some(AsyncDbOperation::TransactionRollback),
        19 => Some(AsyncDbOperation::TransactionCount),
        20 => Some(AsyncDbOperation::ExecCount),
        21 => Some(AsyncDbOperation::TransactionExecCount),
        22 => Some(AsyncDbOperation::MutationReadback),
        23 => Some(AsyncDbOperation::InsertManyReadback),
        _ => None,
    }
}

fn run_operation(input: AsyncDbInput) -> Result<Value, DbRuntimeError> {
    let parameters = decode_parameters(&input.parameters)?;
    match input.operation {
        AsyncDbOperation::TransactionBegin => {
            service::begin_transaction(input.path).map(|id| Value::Number(id.into()))
        }
        AsyncDbOperation::TransactionExec => {
            service::execute_in_transaction(input.path, input.transaction_id, input.sql, parameters)
                .map(|_| Value::Bool(true))
        }
        AsyncDbOperation::TransactionExecCount => {
            service::execute_in_transaction(input.path, input.transaction_id, input.sql, parameters)
                .map(|result| Value::Number(result.rows_affected.into()))
        }
        AsyncDbOperation::MutationReadback => {
            let read_parameters = decode_parameters(&input.count_parameters)?;
            service::mutate_with_readback(
                input.path,
                input.sql,
                parameters,
                input.count_sql,
                read_parameters,
                service::ReadbackOptions {
                    read_before: input.cursor_field == "before",
                    use_inserted_id: input.cursor_field == "insert-id",
                    transaction_id: (input.transaction_id != 0).then_some(input.transaction_id),
                },
            )?
            .as_ref()
            .map(row_json)
            .transpose()
            .map(|value| value.unwrap_or(Value::Null))
        }
        AsyncDbOperation::InsertManyReadback => {
            let read_parameters = decode_parameters(&input.count_parameters)?;
            let readback = decode_insert_many_readback(&input.sql, parameters, read_parameters)?;
            service::insert_many_with_readback(
                input.path,
                readback.mutations,
                readback.readbacks,
                readback.use_inserted_ids,
                (input.transaction_id != 0).then_some(input.transaction_id),
            )?
            .iter()
            .map(row_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array)
        }
        AsyncDbOperation::TransactionOne => service::query_one_in_transaction(
            input.path,
            input.transaction_id,
            input.sql,
            parameters,
        )?
        .as_ref()
        .map(row_json)
        .transpose()
        .map(|value| value.unwrap_or(Value::Null)),
        AsyncDbOperation::TransactionMany => service::query_many_in_transaction(
            input.path,
            input.transaction_id,
            input.sql,
            parameters,
        )?
        .iter()
        .map(row_json)
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array),
        AsyncDbOperation::TransactionCount => {
            let row = service::query_one_in_transaction(
                input.path,
                input.transaction_id,
                input.sql,
                parameters,
            )?
            .ok_or_else(DbRuntimeError::decode)?;
            count_row_value(&row).map(|value| Value::Number(value.into()))
        }
        AsyncDbOperation::TransactionCommit | AsyncDbOperation::TransactionRollback => {
            service::finish_transaction(
                input.path,
                input.transaction_id,
                matches!(input.operation, AsyncDbOperation::TransactionCommit),
            )
            .map(|()| Value::Bool(true))
        }
        AsyncDbOperation::TransactionBatch => {
            let statements = decode_transaction_statements(&input.sql)?;
            service::execute_transaction(input.path, statements)
                .map(|results| Value::Array(results.into_iter().map(exec_result_json).collect()))
        }
        AsyncDbOperation::Exec => {
            service::execute(input.path, input.sql, parameters).map(|_| Value::Bool(true))
        }
        AsyncDbOperation::ExecCount => service::execute(input.path, input.sql, parameters)
            .map(|result| Value::Number(result.rows_affected.into())),
        AsyncDbOperation::One => service::query_one(input.path, input.sql, parameters)?
            .as_ref()
            .map(row_json)
            .transpose()
            .map(|value| value.unwrap_or(Value::Null)),
        AsyncDbOperation::Many => service::query_many(input.path, input.sql, parameters)?
            .iter()
            .map(row_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        AsyncDbOperation::Count => {
            let row = service::query_one(input.path, input.sql, parameters)?
                .ok_or_else(DbRuntimeError::decode)?;
            count_row_value(&row).map(|value| Value::Number(value.into()))
        }
        AsyncDbOperation::Page => {
            let count_parameters = decode_parameters(&input.count_parameters)?;
            let (rows, total) = service::query_page(
                input.path,
                input.sql,
                parameters,
                input.count_sql,
                count_parameters,
                input.page_size,
                input.page_number,
            )?;
            page_json(rows, total, input.page_size, input.page_number)
        }
        AsyncDbOperation::CursorPage => {
            let count_parameters = decode_parameters(&input.count_parameters)?;
            let cursor = if input.after.is_empty() {
                None
            } else {
                if input.after.len() > 4096 {
                    return Err(INVALID_CURSOR);
                }
                let encoded = format!("[{}]", input.after);
                let mut values = decode_parameters(&encoded).map_err(|_| INVALID_CURSOR)?;
                match values.pop() {
                    Some(
                        value @ (DbValue::Bool(_)
                        | DbValue::Signed(_)
                        | DbValue::Unsigned(_)
                        | DbValue::Real(_)
                        | DbValue::Text(_)
                        | DbValue::Instant { .. }),
                    ) => Some(value),
                    _ => return Err(INVALID_CURSOR),
                }
            };
            let (mut rows, total) = service::query_cursor_page(
                input.path,
                input.sql,
                parameters,
                input.count_sql,
                count_parameters,
                input.page_size,
                cursor,
            )?;
            let has_more = rows.len() as u64 > input.page_size;
            if has_more {
                rows.pop();
            }
            cursor_page_json(rows, total, input.page_size, has_more, &input.cursor_field)
        }
        AsyncDbOperation::MongoCount => {
            service::mongo_read(input.path, input.sql, parameters, MongoReadOperation::Count)
        }
        AsyncDbOperation::MongoOne => {
            service::mongo_read(input.path, input.sql, parameters, MongoReadOperation::One)
        }
        AsyncDbOperation::MongoMany => {
            service::mongo_read(input.path, input.sql, parameters, MongoReadOperation::Many)
        }
        AsyncDbOperation::MongoPage => service::mongo_read(
            input.path,
            input.sql,
            parameters,
            MongoReadOperation::Page {
                page_size: input.page_size,
                page_number: input.page_number,
            },
        ),
        AsyncDbOperation::MongoCursorPage => service::mongo_read(
            input.path,
            input.sql,
            parameters,
            MongoReadOperation::CursorPage {
                page_size: input.page_size,
                after: input.after,
            },
        ),
        AsyncDbOperation::MongoMutation => service::mongo_mutate(input.path, input.sql, parameters),
    }
}

fn decode_transaction_statements(source: &str) -> Result<Vec<DbStatement>, DbRuntimeError> {
    let Value::Array(items) =
        serde_json::from_str::<Value>(source).map_err(|_| INVALID_TRANSACTION_BATCH)?
    else {
        return Err(INVALID_TRANSACTION_BATCH);
    };
    if items.is_empty() || items.len() > 500 {
        return Err(INVALID_TRANSACTION_BATCH);
    }
    let mut bound_parameters = 0usize;
    items
        .into_iter()
        .map(|item| {
            let Some(object) = item.as_object() else {
                return Err(INVALID_TRANSACTION_BATCH);
            };
            if object.len() != 2 {
                return Err(INVALID_TRANSACTION_BATCH);
            }
            let Some(sql) = object.get("sql").and_then(Value::as_str) else {
                return Err(INVALID_TRANSACTION_BATCH);
            };
            let Some(Value::Array(parameters)) = object.get("parameters") else {
                return Err(INVALID_TRANSACTION_BATCH);
            };
            if sql.trim().is_empty() {
                return Err(INVALID_TRANSACTION_BATCH);
            }
            bound_parameters = bound_parameters
                .checked_add(parameters.len())
                .ok_or(INVALID_TRANSACTION_BATCH)?;
            if bound_parameters > 999 {
                return Err(TOO_MANY_TRANSACTION_PARAMETERS);
            }
            let encoded = Value::Array(parameters.clone()).to_string();
            let parameters = decode_parameters(&encoded)?;
            Ok(DbStatement::new(sql.into(), parameters))
        })
        .collect()
}

fn decode_insert_many_readback(
    source: &str,
    mutation_parameters: Vec<DbValue>,
    read_parameters: Vec<DbValue>,
) -> Result<InsertManyReadback, DbRuntimeError> {
    let descriptor: Value = serde_json::from_str(source).map_err(|_| INVALID_OPERATION)?;
    let Value::Array(statements) = descriptor
        .get("statements")
        .cloned()
        .ok_or(INVALID_OPERATION)?
    else {
        return Err(INVALID_OPERATION);
    };
    if statements.is_empty() || statements.len() > 500 {
        return Err(INVALID_OPERATION);
    }
    let mut mutation_parameters = std::collections::VecDeque::from(mutation_parameters);
    let mut read_parameters = std::collections::VecDeque::from(read_parameters);
    let mut mutations = Vec::with_capacity(statements.len());
    let mut readbacks = Vec::with_capacity(statements.len());
    let mut use_inserted_ids = Vec::with_capacity(statements.len());
    for statement in statements {
        let sql = statement
            .get("sql")
            .and_then(Value::as_str)
            .filter(|sql| !sql.trim().is_empty())
            .ok_or(INVALID_OPERATION)?;
        let read_sql = statement
            .get("readSql")
            .and_then(Value::as_str)
            .filter(|sql| !sql.trim().is_empty())
            .ok_or(INVALID_OPERATION)?;
        let mutation_count = statement
            .get("parameterCount")
            .and_then(Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
            .ok_or(INVALID_OPERATION)?;
        let read_count = statement
            .get("readParameterCount")
            .and_then(Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
            .ok_or(INVALID_OPERATION)?;
        let use_inserted_id = statement
            .get("useInsertedId")
            .and_then(Value::as_bool)
            .ok_or(INVALID_OPERATION)?;
        if mutation_count > mutation_parameters.len()
            || read_count > read_parameters.len()
            || (use_inserted_id && read_count != 0)
        {
            return Err(INVALID_OPERATION);
        }
        let mut next_mutation_parameters = Vec::with_capacity(mutation_count);
        let mut next_read_parameters = Vec::with_capacity(read_count);
        for _ in 0..mutation_count {
            next_mutation_parameters
                .push(mutation_parameters.pop_front().ok_or(INVALID_OPERATION)?);
        }
        for _ in 0..read_count {
            next_read_parameters.push(read_parameters.pop_front().ok_or(INVALID_OPERATION)?);
        }
        mutations.push(DbStatement::new(
            sql.to_string().into_boxed_str(),
            next_mutation_parameters,
        ));
        readbacks.push(DbStatement::new(
            read_sql.to_string().into_boxed_str(),
            next_read_parameters,
        ));
        use_inserted_ids.push(use_inserted_id);
    }
    if !mutation_parameters.is_empty() || !read_parameters.is_empty() {
        return Err(INVALID_OPERATION);
    }
    Ok(InsertManyReadback {
        mutations,
        readbacks,
        use_inserted_ids,
    })
}

fn exec_result_json(result: DbExecResult) -> Value {
    let mut object = Map::new();
    object.insert(
        "rowsAffected".to_string(),
        Value::Number(result.rows_affected.into()),
    );
    object.insert(
        "lastInsertRowId".to_string(),
        Value::Number(result.last_insert_rowid.into()),
    );
    Value::Object(object)
}

fn count_row_value(row: &DbRow) -> Result<u64, DbRuntimeError> {
    match row.values().first() {
        Some(DbValue::Signed(value)) => u64::try_from(*value).map_err(|_| DbRuntimeError::decode()),
        Some(DbValue::Unsigned(value)) => Ok(*value),
        _ => Err(DbRuntimeError::decode()),
    }
}

fn page_json(
    rows: Vec<DbRow>,
    total: u64,
    page_size: u64,
    page_number: u64,
) -> Result<Value, DbRuntimeError> {
    let items = rows.iter().map(row_json).collect::<Result<Vec<_>, _>>()?;
    let mut object = Map::new();
    object.insert("items".to_string(), Value::Array(items));
    object.insert("total".to_string(), Value::Number(total.into()));
    object.insert("pageSize".to_string(), Value::Number(page_size.into()));
    object.insert("page".to_string(), Value::Number(page_number.into()));
    object.insert(
        "totalPages".to_string(),
        Value::Number(total.div_ceil(page_size).into()),
    );
    Ok(Value::Object(object))
}

fn cursor_page_json(
    rows: Vec<DbRow>,
    total: u64,
    page_size: u64,
    has_more: bool,
    cursor_field: &str,
) -> Result<Value, DbRuntimeError> {
    let next_cursor = if has_more {
        let row = rows.last().ok_or(INVALID_CURSOR)?;
        let unsigned_cursor_field = format!("__tyrus_u64__{cursor_field}");
        let index = row
            .columns()
            .iter()
            .position(|column| column == cursor_field || column == &unsigned_cursor_field)
            .ok_or(INVALID_CURSOR)?;
        let value = row.values().get(index).ok_or(INVALID_CURSOR)?;
        let value = match value {
            DbValue::Bool(value) => Value::Bool(*value),
            DbValue::Signed(value) => Value::from(*value),
            DbValue::Unsigned(value) => Value::from(*value),
            DbValue::Text(value) if row.columns()[index].starts_with("__tyrus_u64__") => {
                Value::from(value.parse::<u64>().map_err(|_| INVALID_CURSOR)?)
            }
            DbValue::Real(value) => serde_json::Number::from_f64(*value)
                .map(Value::Number)
                .ok_or(INVALID_CURSOR)?,
            DbValue::Text(value) => Value::String(value.clone()),
            DbValue::Instant { .. } => return Err(INVALID_CURSOR),
            DbValue::Null | DbValue::Blob(_) | DbValue::Json(_) => return Err(INVALID_CURSOR),
        };
        Some(serde_json::to_string(&value).map_err(|_| INVALID_CURSOR)?)
    } else {
        None
    };
    let items = rows.iter().map(row_json).collect::<Result<Vec<_>, _>>()?;
    let mut object = Map::new();
    object.insert("items".to_string(), Value::Array(items));
    object.insert("total".to_string(), Value::Number(total.into()));
    object.insert("pageSize".to_string(), Value::Number(page_size.into()));
    object.insert("hasMore".to_string(), Value::Bool(has_more));
    if let Some(cursor) = next_cursor {
        object.insert("nextCursor".to_string(), Value::String(cursor));
    }
    Ok(Value::Object(object))
}

fn fulfill_envelope(promise: *mut TyrusPromise, result: Result<Value, DbRuntimeError>) {
    let mut envelope = Map::new();
    match result {
        Ok(value) => {
            envelope.insert("ok".to_string(), Value::Bool(true));
            envelope.insert("value".to_string(), value);
        }
        Err(error) => {
            envelope.insert("ok".to_string(), Value::Bool(false));
            let mut details = Map::new();
            details.insert("code".to_string(), Value::String(error.code.to_string()));
            details.insert(
                "message".to_string(),
                Value::String(error.message.to_string()),
            );
            envelope.insert("error".to_string(), Value::Object(details));
        }
    }
    let payload = encode_result(&Value::Object(envelope)).unwrap_or_else(|_| {
        TyrusString::from_str_owned(
            "{\"ok\":false,\"error\":{\"code\":\"ERR_DB_RESULT_TOO_LARGE\",\"message\":\"database result exceeds the supported string size\"}}",
        )
        .unwrap_or_else(|| TyrusString::literal(PAYLOAD_TOO_LARGE.message))
    });
    let payload = Box::into_raw(Box::new(payload)).cast::<c_void>();
    unsafe {
        crate::tyrus_promise_fulfill_owned(promise, payload, Some(drop_payload));
    }
}

unsafe extern "C" fn drop_payload(payload: *mut c_void) {
    if !payload.is_null() {
        let value = unsafe { Box::from_raw(payload.cast::<TyrusString>()) };
        unsafe { value.release() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_cursor_tokens_preserve_full_u64_values() {
        let row = DbRow::with_columns(
            vec!["__tyrus_u64__id".to_string()],
            vec![DbValue::Text(u64::MAX.to_string())],
        );
        let page = cursor_page_json(vec![row], 1, 1, true, "id").unwrap();
        assert_eq!(page["nextCursor"], serde_json::json!(u64::MAX.to_string()));
        assert_eq!(page["items"][0]["id"], serde_json::json!(u64::MAX));
    }

    fn lock_database_service() -> std::sync::MutexGuard<'static, ()> {
        crate::db::TEST_SERVICE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn transaction_batch_decoder_builds_parameterized_statements() {
        let statements = decode_transaction_statements(
            r#"[{"sql":"INSERT INTO users (name) VALUES (?)","parameters":["Ada"]}]"#,
        )
        .unwrap();
        assert_eq!(statements.len(), 1);
        let (sql, parameters) = statements[0].parts();
        assert_eq!(sql, "INSERT INTO users (name) VALUES (?)");
        assert!(matches!(parameters, [DbValue::Text(name)] if name == "Ada"));
    }

    #[test]
    fn transaction_batch_decoder_rejects_empty_batches_and_parameter_overflow() {
        assert_eq!(
            decode_transaction_statements("[]").err().unwrap().code,
            "ERR_DB_INVALID_TRANSACTION"
        );
        let parameters = vec!["null"; 1000].join(",");
        let source = format!(r#"[{{"sql":"SELECT ?","parameters":[{parameters}]}}]"#);
        assert_eq!(
            decode_transaction_statements(&source).err().unwrap().code,
            "ERR_DB_PARAMETER_LIMIT"
        );
    }

    #[test]
    fn transaction_batch_dispatch_commits_and_rolls_back_atomically() {
        let _guard = lock_database_service();
        static NEXT_TABLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let table = format!(
            "transaction_batch_{}_{}",
            std::process::id(),
            NEXT_TABLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let path = ":memory:".to_string();
        let execute_batch = |items: Value| {
            run_operation(AsyncDbInput {
                operation: AsyncDbOperation::TransactionBatch,
                path: path.clone(),
                sql: items.to_string(),
                parameters: "[]".to_string(),
                count_sql: String::new(),
                count_parameters: "[]".to_string(),
                after: String::new(),
                cursor_field: String::new(),
                page_size: 0,
                page_number: 0,
                transaction_id: 0,
            })
        };
        let statement = |sql: String, parameters: Vec<Value>| serde_json::json!({ "sql": sql, "parameters": parameters });

        execute_batch(Value::Array(vec![
            statement(
                format!("CREATE TABLE {table} (value INTEGER UNIQUE)"),
                Vec::new(),
            ),
            statement(
                format!("INSERT INTO {table} VALUES (?)"),
                vec![Value::from(1)],
            ),
            statement(
                format!("INSERT INTO {table} VALUES (?)"),
                vec![Value::from(2)],
            ),
        ]))
        .unwrap();

        let failure = execute_batch(Value::Array(vec![
            statement(
                format!("INSERT INTO {table} VALUES (?)"),
                vec![Value::from(3)],
            ),
            statement(
                format!("INSERT INTO {table} VALUES (?)"),
                vec![Value::from(1)],
            ),
        ]))
        .unwrap_err();
        assert_eq!(failure.code, "ERR_DB_SQLITE_CONSTRAINT");

        let rows = service::query_many(
            path,
            format!("SELECT value FROM {table} ORDER BY value"),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].values(), &[DbValue::Signed(1)]);
        assert_eq!(rows[1].values(), &[DbValue::Signed(2)]);
    }

    #[test]
    fn async_transaction_operations_share_a_handle_and_rollback_on_request() {
        let _guard = lock_database_service();
        static NEXT_TABLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let table = format!(
            "async_transaction_{}_{}",
            std::process::id(),
            NEXT_TABLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let path = ":memory:".to_string();
        service::execute(
            path.clone(),
            format!("CREATE TABLE {table} (value INTEGER)"),
            Vec::new(),
        )
        .unwrap();
        let make_input = |operation, transaction_id, sql: String, parameters: &str| AsyncDbInput {
            operation,
            path: path.clone(),
            sql,
            parameters: parameters.to_string(),
            count_sql: String::new(),
            count_parameters: "[]".to_string(),
            after: String::new(),
            cursor_field: String::new(),
            page_size: 0,
            page_number: 0,
            transaction_id,
        };

        let transaction_id = run_operation(make_input(
            AsyncDbOperation::TransactionBegin,
            0,
            String::new(),
            "[]",
        ))
        .unwrap()
        .as_u64()
        .unwrap();
        run_operation(make_input(
            AsyncDbOperation::TransactionExec,
            transaction_id,
            format!("INSERT INTO {table} VALUES (?)"),
            "[42]",
        ))
        .unwrap();
        let rows = run_operation(make_input(
            AsyncDbOperation::TransactionMany,
            transaction_id,
            format!("SELECT value FROM {table}"),
            "[]",
        ))
        .unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1);
        run_operation(make_input(
            AsyncDbOperation::TransactionRollback,
            transaction_id,
            String::new(),
            "[]",
        ))
        .unwrap();

        let rows =
            service::query_many(path, format!("SELECT value FROM {table}"), Vec::new()).unwrap();
        assert!(rows.is_empty());
    }
}
