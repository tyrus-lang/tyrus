---
title: updateMany
description: Update all model records that match a non-empty filter.
---

# Model.updateMany

Call `User.updateMany(filter, changes)` on an imported model. The filter must be a non-empty object, and `changes` must contain at least one known model field. The `@id` field cannot be changed.

The update is parameterized and applies to every record matching the filter. It returns `Promise<Result<u64, DbError>>`; the success value is the number of updated records. Database constraint and connection failures are recoverable `DbError` values.

This fragment assumes an imported `User` model with `id` marked `@id` and `email` and `displayName` fields.

<!-- @include: ../../.fragments/modules/db/update-many/example.md -->

Use [`Model.updateOne(filter, changes)`](./update-one.md) when the filter identifies one `@id` or `@unique` field.
