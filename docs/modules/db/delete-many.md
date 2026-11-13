---
title: deleteMany
description: Delete all model records that match a non-empty filter.
---

# Model.deleteMany

Call `User.deleteMany(filter)` on an imported model. The filter must be a non-empty object. Every filter value is parameterized, and the compiler checks field names and value types against the model and schema snapshot.

The delete applies to every record matching the filter and returns `Promise<Result<u64, DbError>>`; the success value is the number of deleted records. Database constraint and connection failures are recoverable `DbError` values.

This fragment assumes an imported `User` model with an `email` field.

<!-- @include: ../../.fragments/modules/db/delete-many/example.md -->

Use [`Model.deleteOne(filter)`](./delete-one.md) when the filter identifies one `@id` or `@unique` field.
