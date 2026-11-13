---
title: DbError
description: Inspect recoverable errors from tyrus:db operations.
---

# DbError

`DbError` is the structured error type for recoverable database failures.

<SignatureBlock label="Signature">interface DbError { readonly code: string; readonly message: string }</SignatureBlock>

## Fields

| Field | Type | Purpose |
| --- | --- | --- |
| `code` | `string` | Stable identifier for programmatic handling, such as `ERR_DB_QUERY_FAILED`. |
| `message` | `string` | Human-readable explanation of the failure. |

Database operations return `Promise<Result<T, DbError>>`. Await each call to receive the `Result`, then handle failures with `onfail`, `?`, or pattern matching. The `tyrus:db` API does not use `throws`.

## Runtime codes

| Code | Meaning |
| --- | --- |
| `ERR_DB_CONNECTION_FAILED` | The configured database backend could not be connected. |
| `ERR_DB_CONNECTION_TIMEOUT` | The PostgreSQL, MySQL, or MongoDB connection attempt exceeded `connectTimeoutMs`. |
| `ERR_DB_POOL_TIMEOUT` | A PostgreSQL or MySQL pool connection, or a MongoDB runtime operation slot, was unavailable before `acquireTimeoutMs`. |
| `ERR_DB_QUERY_TIMEOUT` | A PostgreSQL, MySQL, or MongoDB database operation exceeded `queryTimeoutMs`. |
| `ERR_DB_BUSY` | The bounded database worker queue already contains 128 pending requests. |
| `ERR_DB_CONNECTION_CONFIGURATION` | The environment variable named by `database.connectionEnv` is missing. |
| `ERR_DB_INVALID_CONFIGURATION` | The runtime database target is malformed or does not match a supported configuration. |
| `ERR_DB_INVALID_CONNECTION` | The connection variable does not contain a valid backend URL or URI. The connection value is never included in the error message. |
| `ERR_DB_TLS_CONFIGURATION` | A configured database TLS file conflicts with the connection URL or does not allow certificate and hostname verification. |
| `ERR_DB_BACKEND_UNAVAILABLE` | The configured backend does not have a runtime driver in this build. |
| `ERR_DB_INVALID_SQL` | SQL could not be prepared for the configured backend. |
| `ERR_DB_FEATURE_UNSUPPORTED` | The configured backend does not support the requested database feature. |
| `ERR_DB_SQLITE_BUSY` | SQLite could not acquire the required lock before the timeout. |
| `ERR_DB_SQLITE_CONSTRAINT` | A database constraint rejected the change. |
| `ERR_DB_EXECUTE_FAILED` | The configured database backend rejected or could not execute the statement. |
| `ERR_DB_INVALID_PARAMETERS` | Parameters could not be encoded as supported database values. |
| `ERR_DB_PARAMETER_LIMIT` | A statement attempted to bind more than 999 parameters. |
| `ERR_DB_DECODE_FAILED` | The database returned a value that could not be decoded to a supported type. |
| `ERR_DB_VALUE_OUT_OF_RANGE` | A value could not be represented by the target database type. |
| `ERR_DB_DOCUMENT_TOO_LARGE` | A serialized MongoDB BSON document or filter exceeds 16 MiB. |
| `ERR_DB_VALUE_TOO_DEEP` | A MongoDB structured value exceeds 100 nested levels. |
| `ERR_DB_INVALID_TEMPORAL_VALUE` | A MongoDB `Instant` has invalid `seconds` or `nanoseconds` fields. |
| `ERR_DB_INVALID_MODEL_VALUE` | A MongoDB Model value is missing a required field or does not match its declared field type. |
