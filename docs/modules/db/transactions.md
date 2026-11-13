---
title: Transactions
description: Explicit SQL transaction handles for grouping queries and Model operations.
---

# Database transactions

Use `db.beginTransaction()` to start a transaction, pass its handle as `{ transaction }` to `db.query` or a non-paginated Model operation, then finish with `db.commitTransaction(transaction)` or `db.rollbackTransaction(transaction)`. Each call is asynchronous and returns `Promise<Result<T, DbError>>`; handle every result with `onfail`, `?`, or pattern matching.

`db.beginTransaction()` returns `Promise<Result<DbTransaction, DbError>>`; commit and rollback return `Promise<Result<boolean, DbError>>`, where `true` confirms the transaction was finished. Import `DbTransaction` from `tyrus:db` only when writing it in an explicit type annotation; inferred handles do not need a separate import.

## Scope

The transaction handle pins SQL queries and non-paginated Model operations to one connection and transaction for SQLite, PostgreSQL, or MySQL. Pass `{ transaction }` as the options object to `findOne` and `count`, include it in the existing options object for `findMany`, and pass it as the final options argument to Model mutations. Paginated `findMany` manages its own consistent read transaction and rejects an explicit handle. MongoDB transactions are not available through this API.

Transactions are explicit: a successful sequence must commit, while an application-detected failure should roll back before returning the error. If a SQL query or Model operation using the handle returns an error, Tyrus rolls back the transaction and closes the handle automatically; later operations with that handle return `ERR_DB_INVALID_TRANSACTION`. If a transaction is abandoned without either operation, the runtime rolls it back when the database service shuts down; do not use this as normal control flow.

The handle is opaque and must be passed back unchanged. It is not a row ID or a security token, and applications should not expose it to clients.

## Model example

Pass `{ transaction }` as the final options argument to a mutation. Reads can use the same handle in their existing options object.

<!-- @include: ../../.fragments/modules/db/transactions/example.md -->
