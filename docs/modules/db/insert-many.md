---
title: insertMany
description: Insert a bounded batch of model records.
---

# Model.insertMany

Call `User.insertMany(rows)` on an imported model to insert multiple records in one backend operation. Every row must provide the same field names in the same order. SQL backends bind values as parameters, and a constraint failure rejects the operation with a recoverable `DbError`.

The return type is `Promise<Result<User[], DbError>>`; await it before using the `Result`. On success, the inserted records are returned as `User[]`, including generated identifiers and timestamps. A batch must contain between 1 and 500 rows. SQLite, PostgreSQL, and MySQL use a conservative Tyrus limit of 999 bound SQL parameters, so their effective row limit is `min(500, floor(999 / fieldCount))`; for example, rows with six supplied fields allow at most 166 records per call. MongoDB sends BSON documents rather than SQL parameters, so this SQL limit does not apply; it accepts up to 500 documents, subject to the 16 MiB BSON limit for each document.

Required non-null columns without a database default must be supplied in every row. Nullable columns, columns with a default in the local database snapshot, and the model's single `@id` primary key may be omitted. An omitted primary key is generated only when the configured database table schema defines an identity or default for it.

SQL backends execute the batch as one statement. MongoDB validates every document before sending the batch and uses an ordered insert operation, but a server-side failure can still leave documents inserted before the failure; `insertMany` does not promise rollback across backends. On success, all inserted resources are returned in input order, including generated identifiers. MongoDB may still report a recoverable operation failure after partial server-side progress.

This fragment assumes an imported `User` model with an `email` field.

<!-- @include: ../../.fragments/modules/db/insert-many/example.md -->

Use [`Model.insertOne(data)`](./insert-one.md) when inserting one record.
