use super::service;
use super::sqlite::DbRuntimeError;
use super::{DbRow, DbValue};
use crate::abi::DbErrorTaken;
use jiff::Timestamp;
use serde_json::Value;
use tyrus_string::TyrusString;

const INVALID_PARAMETERS: DbRuntimeError = DbRuntimeError {
    code: "ERR_DB_INVALID_PARAMETERS",
    message: "database parameters must be a JSON array of supported scalar values",
};
const TOO_MANY_PARAMETERS: DbRuntimeError = DbRuntimeError {
    code: "ERR_DB_PARAMETER_LIMIT",
    message: "database statements cannot bind more than 999 parameters",
};
const MAX_BOUND_PARAMETERS: usize = 999;
const INVALID_RESULT: DbRuntimeError = DbRuntimeError {
    code: "ERR_DB_DECODE_FAILED",
    message: "database returned a value that could not be decoded",
};

/// Executes a statement with an owned, JSON-encoded parameter list.
///
/// # Safety
///
/// The string arguments must be valid borrowed Tyrus strings. All output
/// pointers must refer to initialized, writable values for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_db_query_exec(
    path: *const TyrusString,
    sql: *const TyrusString,
    parameters: *const TyrusString,
    rows_affected: *mut u64,
    last_insert_rowid: *mut i64,
    error: *mut DbErrorTaken,
) -> i32 {
    if path.is_null()
        || sql.is_null()
        || parameters.is_null()
        || rows_affected.is_null()
        || last_insert_rowid.is_null()
        || error.is_null()
    {
        return 0;
    }
    let result = (|| {
        let parameters = decode_parameters(unsafe { (&*parameters).as_str() })?;
        service::execute(
            unsafe { (&*path).as_str() }.to_owned(),
            unsafe { (&*sql).as_str() }.to_owned(),
            parameters,
        )
    })();
    match result {
        Ok(result) => {
            unsafe {
                *rows_affected = result.rows_affected;
                *last_insert_rowid = result.last_insert_rowid;
            }
            1
        }
        Err(failure) => {
            unsafe { write_error(error, failure) };
            0
        }
    }
}

/// Returns one row as a JSON object keyed by selected column names, or JSON null when absent.
///
/// # Safety
///
/// The string arguments must be valid borrowed Tyrus strings. Both output
/// pointers must refer to initialized, writable values for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_db_get_one(
    path: TyrusString,
    sql: TyrusString,
    parameters: TyrusString,
    output: *mut TyrusString,
    error: *mut DbErrorTaken,
) -> i32 {
    if output.is_null() || error.is_null() {
        return 0;
    }
    let result = (|| {
        let parameters = decode_parameters(unsafe { parameters.as_str() })?;
        let row = service::query_one(
            unsafe { path.as_str() }.to_owned(),
            unsafe { sql.as_str() }.to_owned(),
            parameters,
        )?;
        let value = row
            .as_ref()
            .map(row_json)
            .transpose()?
            .unwrap_or(Value::Null);
        encode_result(&value)
    })();
    finish_json_result(result, output, error)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_db_count(
    path: TyrusString,
    sql: TyrusString,
    parameters: TyrusString,
    output: *mut u64,
    error: *mut DbErrorTaken,
) -> i32 {
    if output.is_null() || error.is_null() {
        return 0;
    }
    let result = (|| {
        let parameters = decode_parameters(unsafe { parameters.as_str() })?;
        let row = service::query_one(
            unsafe { path.as_str() }.to_owned(),
            unsafe { sql.as_str() }.to_owned(),
            parameters,
        )?
        .ok_or(INVALID_RESULT)?;
        match row.values().first() {
            Some(DbValue::Signed(value)) => u64::try_from(*value).map_err(|_| INVALID_RESULT),
            _ => Err(INVALID_RESULT),
        }
    })();
    match result {
        Ok(value) => {
            unsafe { output.write(value) };
            1
        }
        Err(failure) => {
            unsafe { write_error(error, failure) };
            0
        }
    }
}

/// Returns rows as a JSON array of objects keyed by selected column names.
///
/// # Safety
///
/// The string arguments must be valid borrowed Tyrus strings. Both output
/// pointers must refer to initialized, writable values for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_db_get_many(
    path: TyrusString,
    sql: TyrusString,
    parameters: TyrusString,
    output: *mut TyrusString,
    error: *mut DbErrorTaken,
) -> i32 {
    if output.is_null() || error.is_null() {
        return 0;
    }
    let result = (|| {
        let parameters = decode_parameters(unsafe { parameters.as_str() })?;
        let rows = service::query_many(
            unsafe { path.as_str() }.to_owned(),
            unsafe { sql.as_str() }.to_owned(),
            parameters,
        )?;
        let values = rows.iter().map(row_json).collect::<Result<Vec<_>, _>>()?;
        encode_result(&Value::Array(values))
    })();
    finish_json_result(result, output, error)
}

/// Executes a paginated model query and its total count in one SQLite read transaction.
///
/// # Safety
///
/// The string arguments must be valid borrowed Tyrus strings. Both output
/// pointers must refer to initialized, writable values for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_db_get_page(
    path: TyrusString,
    rows_sql: TyrusString,
    rows_parameters: TyrusString,
    count_sql: TyrusString,
    count_parameters: TyrusString,
    page_size: u64,
    page_number: u64,
    output: *mut TyrusString,
    error: *mut DbErrorTaken,
) -> i32 {
    if output.is_null() || error.is_null() {
        return 0;
    }
    let result = (|| {
        if page_size == 0 || page_number == 0 {
            return Err(DbRuntimeError {
                code: "ERR_DB_INVALID_PAGINATION",
                message: "page and pageSize must be greater than zero",
            });
        }
        let rows_parameters = decode_parameters(unsafe { rows_parameters.as_str() })?;
        let count_parameters = decode_parameters(unsafe { count_parameters.as_str() })?;
        let (rows, total) = service::query_page(
            unsafe { path.as_str() }.to_owned(),
            unsafe { rows_sql.as_str() }.to_owned(),
            rows_parameters,
            unsafe { count_sql.as_str() }.to_owned(),
            count_parameters,
            page_size,
            page_number,
        )?;
        let items = rows.iter().map(row_json).collect::<Result<Vec<_>, _>>()?;
        let mut page_json = serde_json::Map::new();
        page_json.insert("items".to_string(), Value::Array(items));
        page_json.insert("total".to_string(), Value::Number(total.into()));
        page_json.insert("pageSize".to_string(), Value::Number(page_size.into()));
        page_json.insert("page".to_string(), Value::Number(page_number.into()));
        let total_pages = total.div_ceil(page_size);
        page_json.insert("totalPages".to_string(), Value::Number(total_pages.into()));
        encode_result(&Value::Object(page_json))
    })();
    finish_json_result(result, output, error)
}

pub(super) fn decode_parameters(source: &str) -> Result<Vec<DbValue>, DbRuntimeError> {
    let Value::Array(values) =
        serde_json::from_str::<Value>(source).map_err(|_| INVALID_PARAMETERS)?
    else {
        return Err(INVALID_PARAMETERS);
    };
    if values.len() > MAX_BOUND_PARAMETERS {
        return Err(TOO_MANY_PARAMETERS);
    }
    values
        .into_iter()
        .map(|value| match value {
            Value::Null => Ok(DbValue::Null),
            Value::Bool(value) => Ok(DbValue::Bool(value)),
            Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    Ok(DbValue::Signed(value))
                } else if let Some(value) = value.as_u64() {
                    Ok(DbValue::Unsigned(value))
                } else {
                    value.as_f64().map(DbValue::Real).ok_or(INVALID_PARAMETERS)
                }
            }
            Value::String(value) => Ok(DbValue::Text(value)),
            Value::Array(bytes) => bytes
                .into_iter()
                .map(|byte| {
                    byte.as_u64()
                        .and_then(|value| u8::try_from(value).ok())
                        .ok_or(INVALID_PARAMETERS)
                })
                .collect::<Result<Vec<_>, _>>()
                .map(DbValue::Blob),
            Value::Object(mut value) if value.len() == 1 && value.contains_key("$tyrusJson") => {
                value
                    .remove("$tyrusJson")
                    .map(DbValue::Json)
                    .ok_or(INVALID_PARAMETERS)
            }
            Value::Object(mut value)
                if value.len() == 1 && value.contains_key("$tyrusInstantParam") =>
            {
                let Some(Value::Object(mut instant)) = value.remove("$tyrusInstantParam") else {
                    return Err(INVALID_PARAMETERS);
                };
                if instant.len() != 2 {
                    return Err(INVALID_PARAMETERS);
                }
                let seconds = instant
                    .remove("seconds")
                    .and_then(|value| value.as_i64())
                    .ok_or(INVALID_PARAMETERS)?;
                let nanoseconds = instant
                    .remove("nanoseconds")
                    .and_then(|value| value.as_u64())
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or(INVALID_PARAMETERS)?;
                Timestamp::new(
                    seconds,
                    i32::try_from(nanoseconds).map_err(|_| INVALID_PARAMETERS)?,
                )
                .map_err(|_| INVALID_PARAMETERS)?;
                Ok(DbValue::Instant {
                    seconds,
                    nanoseconds,
                })
            }
            Value::Object(_) => Err(INVALID_PARAMETERS),
        })
        .collect()
}

pub(super) fn row_json(row: &DbRow) -> Result<Value, DbRuntimeError> {
    let mut object = serde_json::Map::with_capacity(row.values().len());
    for (name, value) in row.columns().iter().zip(row.values()) {
        if let Some(field) = name.strip_prefix("__tyrus_u64__") {
            let number = match value {
                DbValue::Null => continue,
                DbValue::Signed(value) => u64::try_from(*value).map_err(|_| INVALID_RESULT)?,
                DbValue::Unsigned(value) => *value,
                DbValue::Text(value) => value.parse::<u64>().map_err(|_| INVALID_RESULT)?,
                _ => return Err(INVALID_RESULT),
            };
            object.insert(field.to_string(), Value::from(number));
            continue;
        }
        if let Some(field) = name.strip_prefix("__tyrus_instant__") {
            let instant = match value {
                DbValue::Null => continue,
                DbValue::Text(value) => decode_instant(value)?,
                _ => return Err(INVALID_RESULT),
            };
            object.insert(field.to_string(), instant);
            continue;
        }
        if let Some(field) = name.strip_prefix("__tyrus_json__") {
            let related = match value {
                DbValue::Null => Value::Null,
                DbValue::Text(value) => {
                    let mut related = serde_json::from_str(value).map_err(|_| INVALID_RESULT)?;
                    decode_related_instants(&mut related)?;
                    related
                }
                _ => return Err(INVALID_RESULT),
            };
            object.insert(field.to_string(), related);
            continue;
        }
        let value = match value {
            DbValue::Null => continue,
            DbValue::Bool(value) => Value::Bool(*value),
            DbValue::Signed(value) => Value::from(*value),
            DbValue::Unsigned(value) => Value::from(*value),
            DbValue::Real(value) => serde_json::Number::from_f64(*value)
                .map(Value::Number)
                .ok_or(INVALID_RESULT)?,
            DbValue::Text(value) => Value::String(value.clone()),
            DbValue::Blob(value) => Value::Array(value.iter().copied().map(Value::from).collect()),
            DbValue::Instant {
                seconds,
                nanoseconds,
            } => {
                let mut instant = serde_json::Map::with_capacity(2);
                instant.insert("seconds".to_string(), Value::from(*seconds));
                instant.insert("nanoseconds".to_string(), Value::from(*nanoseconds));
                Value::Object(instant)
            }
            DbValue::Json(value) => value.clone(),
        };
        object.insert(name.clone(), value);
    }
    Ok(Value::Object(object))
}

fn decode_related_instants(value: &mut Value) -> Result<(), DbRuntimeError> {
    match value {
        Value::Array(values) => {
            for value in values {
                decode_related_instants(value)?;
            }
        }
        Value::Object(fields) => {
            let instant_fields = fields
                .keys()
                .filter_map(|name| {
                    name.strip_prefix("__tyrus_instant__")
                        .map(|field| (name.clone(), field.to_string()))
                })
                .collect::<Vec<_>>();
            for (encoded, field) in instant_fields {
                let value = fields.remove(&encoded).ok_or(INVALID_RESULT)?;
                let decoded = match value {
                    Value::Null => Value::Null,
                    Value::String(value) => decode_instant(&value)?,
                    _ => return Err(INVALID_RESULT),
                };
                fields.insert(field, decoded);
            }
            for value in fields.values_mut() {
                decode_related_instants(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn decode_instant(value: &str) -> Result<Value, DbRuntimeError> {
    let normalized;
    let value = if let Some(separator) = value.find(' ').filter(|_| !value.contains('T')) {
        normalized = format!("{}T{}Z", &value[..separator], &value[separator + 1..]);
        normalized.as_str()
    } else {
        value
    };
    let timestamp = value.parse::<Timestamp>().map_err(|_| INVALID_RESULT)?;
    let mut instant = serde_json::Map::with_capacity(2);
    instant.insert("seconds".to_string(), Value::from(timestamp.as_second()));
    instant.insert(
        "nanoseconds".to_string(),
        Value::from(timestamp.subsec_nanosecond() as u32),
    );
    Ok(Value::Object(instant))
}

pub(super) fn encode_result(value: &Value) -> Result<TyrusString, DbRuntimeError> {
    let text = serde_json::to_string(value).map_err(|_| INVALID_RESULT)?;
    TyrusString::from_str_owned(&text).ok_or(INVALID_RESULT)
}

fn finish_json_result(
    result: Result<TyrusString, DbRuntimeError>,
    output: *mut TyrusString,
    error: *mut DbErrorTaken,
) -> i32 {
    match result {
        Ok(value) => {
            unsafe { output.write(value) };
            1
        }
        Err(failure) => {
            unsafe { write_error(error, failure) };
            0
        }
    }
}

unsafe fn write_error(output: *mut DbErrorTaken, failure: DbRuntimeError) {
    unsafe {
        (*output).code = TyrusString::literal(failure.code);
        (*output).message = TyrusString::literal(failure.message);
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
    fn null_database_columns_decode_as_absent_optional_fields() {
        let row = DbRow::with_columns(
            vec!["total".to_string(), "sum".to_string()],
            vec![DbValue::Signed(0), DbValue::Null],
        );
        assert_eq!(row_json(&row).unwrap().to_string(), "{\"total\":0}");
    }

    #[test]
    fn typed_unsigned_projection_decodes_the_full_u64_range() {
        let row = DbRow::with_columns(
            vec!["__tyrus_u64__id".to_string()],
            vec![DbValue::Text(u64::MAX.to_string())],
        );
        assert_eq!(row_json(&row).unwrap(), serde_json::json!({"id": u64::MAX}));
    }

    fn error_slot() -> DbErrorTaken {
        DbErrorTaken {
            code: TyrusString::null(),
            message: TyrusString::null(),
        }
    }

    #[test]
    fn ffi_exec_and_queries_round_trip_parameters_and_rows() {
        let _guard = lock_database_service();
        let create = TyrusString::literal(
            "CREATE TABLE ffi_items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        );
        let mut rows_affected = 0;
        let mut last_insert_rowid = 0;
        let mut error = error_slot();
        assert_eq!(
            unsafe {
                tyrus_db_query_exec(
                    &TyrusString::literal(":memory:"),
                    &create,
                    &TyrusString::literal("[]"),
                    &mut rows_affected,
                    &mut last_insert_rowid,
                    &mut error,
                )
            },
            1
        );

        let insert = TyrusString::literal("INSERT INTO ffi_items (name) VALUES (?)");
        let parameter = TyrusString::literal("[\"alpha\"]");
        assert_eq!(
            unsafe {
                tyrus_db_query_exec(
                    &TyrusString::literal(":memory:"),
                    &insert,
                    &parameter,
                    &mut rows_affected,
                    &mut last_insert_rowid,
                    &mut error,
                )
            },
            1
        );
        assert_eq!(rows_affected, 1);
        assert_eq!(last_insert_rowid, 1);

        let query = TyrusString::literal("SELECT name FROM ffi_items WHERE id = ?");
        let id = TyrusString::literal("[1]");
        let mut one = TyrusString::null();
        assert_eq!(
            unsafe {
                tyrus_db_get_one(
                    TyrusString::literal(":memory:"),
                    query,
                    id,
                    &mut one,
                    &mut error,
                )
            },
            1
        );
        assert_eq!(unsafe { one.as_str() }, "{\"name\":\"alpha\"}");
        unsafe { one.release() };

        let query = TyrusString::literal("SELECT name FROM ffi_items ORDER BY id");
        let mut many = TyrusString::null();
        assert_eq!(
            unsafe {
                tyrus_db_get_many(
                    TyrusString::literal(":memory:"),
                    query,
                    TyrusString::literal("[]"),
                    &mut many,
                    &mut error,
                )
            },
            1
        );
        assert_eq!(unsafe { many.as_str() }, "[{\"name\":\"alpha\"}]");
        unsafe { many.release() };
    }

    #[test]
    fn ffi_rejects_unsupported_parameters_with_stable_error() {
        let _guard = lock_database_service();
        let mut rows_affected = 0;
        let mut last_insert_rowid = 0;
        let mut error = error_slot();
        let status = unsafe {
            tyrus_db_query_exec(
                &TyrusString::literal(":memory:"),
                &TyrusString::literal("DELETE FROM ffi_items WHERE id = ?"),
                &TyrusString::literal("[{\"id\":1}]"),
                &mut rows_affected,
                &mut last_insert_rowid,
                &mut error,
            )
        };
        assert_eq!(status, 0);
        assert_eq!(unsafe { error.code.as_str() }, "ERR_DB_INVALID_PARAMETERS");
    }
}
