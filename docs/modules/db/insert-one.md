---
title: insertOne
description: Insert one model record with bound values.
---

# Model.insertOne

Call `User.insertOne(data)` on an imported model to insert one record. Object keys are model field names; the compiler maps them to declared database columns and binds every value as a parameter.

The return type is `Promise<Result<User, DbError>>`; await it before using the `Result`. On success, the inserted `User` is returned with its generated ID and timestamps. Database constraint and connection failures are returned as `Err(error)`.

Required non-null columns without a database default must be supplied. Nullable columns, columns with a default in the local database snapshot, and the model's single `@id` primary key may be omitted. An omitted primary key is generated only when the configured database table schema defines an identity or default for it.

This fragment assumes an imported `User` model with an `email` field.

<!-- @include: ../../.fragments/modules/db/insert-one/example.md -->

Use [`Model.insertMany(rows)`](./insert-many.md) to insert a batch in one statement.
