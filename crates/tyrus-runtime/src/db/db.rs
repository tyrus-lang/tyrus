use super::abi::DbErrorTaken;
use std::ffi::CStr;
use tyrus_string::TyrusString;

#[path = "async_ffi.rs"]
mod async_ffi;
#[path = "ffi.rs"]
mod ffi;
#[path = "mongodb.rs"]
mod mongodb;
#[path = "server.rs"]
mod server;
#[path = "service.rs"]
mod service;
#[path = "sqlite.rs"]
mod sqlite;

#[cfg(test)]
pub(crate) static TEST_SERVICE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn shutdown() {
    service::shutdown();
}

/// Writes a recoverable database error into the current call's ABI output.
/// The code and message must be static UTF-8 C string literals.
///
/// # Safety
///
/// `out` must point to a writable, zero-initialized `DbErrorTaken` for this call.
#[allow(
    dead_code,
    reason = "Database backends introduced in phase 2 write failures through this per-call boundary"
)]
pub(crate) unsafe fn write_db_error(
    out: *mut DbErrorTaken,
    code: &'static CStr,
    message: &'static CStr,
) {
    unsafe {
        (*out).code = TyrusString::literal(
            code.to_str()
                .expect("a static database error code is UTF-8"),
        );
        (*out).message = TyrusString::literal(
            message
                .to_str()
                .expect("a static database error message is UTF-8"),
        );
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(
    dead_code,
    reason = "Database drivers use this owned neutral value representation beginning in phase 2"
)]
pub(crate) enum DbValue {
    Null,
    Bool(bool),
    Signed(i64),
    Unsigned(u64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
    Instant { seconds: i64, nanoseconds: u32 },
    Json(serde_json::Value),
}

#[allow(
    dead_code,
    reason = "Database drivers use these RAII wrappers beginning in phase 2"
)]
pub(crate) struct DbConnection<Driver> {
    driver: Driver,
}

#[allow(
    dead_code,
    reason = "Database drivers use these RAII wrappers beginning in phase 2"
)]
impl<Driver> DbConnection<Driver> {
    pub(crate) fn new(driver: Driver) -> Self {
        Self { driver }
    }

    pub(crate) fn driver(&mut self) -> &mut Driver {
        &mut self.driver
    }
}

#[allow(
    dead_code,
    reason = "Database drivers use these RAII wrappers beginning in phase 2"
)]
#[derive(Clone)]
pub(crate) struct DbStatement {
    sql: Box<str>,
    params: Vec<DbValue>,
}

#[allow(
    dead_code,
    reason = "Database drivers use these RAII wrappers beginning in phase 2"
)]
impl DbStatement {
    pub(crate) fn new(sql: Box<str>, params: Vec<DbValue>) -> Self {
        Self { sql, params }
    }

    pub(crate) fn parts(&self) -> (&str, &[DbValue]) {
        (&self.sql, &self.params)
    }

    pub(crate) fn into_parts(self) -> (Box<str>, Vec<DbValue>) {
        (self.sql, self.params)
    }
}

#[allow(
    dead_code,
    reason = "Database drivers use these RAII wrappers beginning in phase 2"
)]
pub(crate) struct DbRow {
    columns: Vec<String>,
    values: Vec<DbValue>,
}

#[allow(
    dead_code,
    reason = "Database drivers use these RAII wrappers beginning in phase 2"
)]
impl DbRow {
    pub(crate) fn new(values: Vec<DbValue>) -> Self {
        let columns = (0..values.len())
            .map(|index| format!("column{index}"))
            .collect();
        Self { columns, values }
    }

    pub(crate) fn with_columns(columns: Vec<String>, values: Vec<DbValue>) -> Self {
        Self { columns, values }
    }

    pub(crate) fn columns(&self) -> &[String] {
        &self.columns
    }

    pub(crate) fn values(&self) -> &[DbValue] {
        &self.values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_error_slot() -> DbErrorTaken {
        DbErrorTaken {
            code: TyrusString::null(),
            message: TyrusString::null(),
        }
    }

    #[test]
    fn database_error_payload_matches_the_two_string_abi() {
        assert_eq!(
            std::mem::size_of::<DbErrorTaken>(),
            2 * std::mem::size_of::<TyrusString>()
        );
        assert_eq!(
            std::mem::align_of::<DbErrorTaken>(),
            std::mem::align_of::<TyrusString>()
        );
        assert_eq!(std::mem::offset_of!(DbErrorTaken, code), 0);
        assert_eq!(
            std::mem::offset_of!(DbErrorTaken, message),
            std::mem::size_of::<TyrusString>()
        );
    }

    #[test]
    fn database_error_payload_uses_both_static_fields() {
        let mut error = empty_error_slot();
        unsafe {
            write_db_error(
                &mut error,
                c"ERR_DB_CONNECTION_FAILED",
                c"database connection failed",
            );
        }

        assert_eq!(unsafe { error.code.as_str() }, "ERR_DB_CONNECTION_FAILED");
        assert_eq!(
            unsafe { error.message.as_str() },
            "database connection failed"
        );
        assert!(!error.code.is_owned());
        assert!(!error.message.is_owned());
    }

    #[test]
    fn database_errors_use_independent_per_call_slots() {
        let mut first = empty_error_slot();
        let mut second = empty_error_slot();
        unsafe {
            write_db_error(&mut first, c"ERR_DB_QUERY_FAILED", c"query failed");
            write_db_error(&mut second, c"ERR_DB_TIMEOUT", c"query timed out");
        }

        assert_eq!(unsafe { first.code.as_str() }, "ERR_DB_QUERY_FAILED");
        assert_eq!(unsafe { second.code.as_str() }, "ERR_DB_TIMEOUT");
        assert_eq!(unsafe { first.message.as_str() }, "query failed");
        assert_eq!(unsafe { second.message.as_str() }, "query timed out");
    }

    #[test]
    fn statement_and_row_own_their_values() {
        let statement =
            DbStatement::new("SELECT ?".into(), vec![DbValue::Text("owned".to_string())]);
        assert_eq!(statement.parts().0, "SELECT ?");
        assert_eq!(statement.parts().1, &[DbValue::Text("owned".to_string())]);

        let row = DbRow::new(vec![DbValue::Null, DbValue::Blob(vec![1, 2, 3])]);
        assert_eq!(row.values().len(), 2);
        assert_eq!(row.values()[0], DbValue::Null);

        let mut connection = DbConnection::new(7_u8);
        assert_eq!(*connection.driver(), 7);
    }
}
