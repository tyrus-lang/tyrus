---
title: updateOne
description: Update one model record selected by a unique field.
---

# Model.updateOne

Call `User.updateOne(filter, changes)` on an imported model. The filter must contain an equality condition for a field marked `@id` or `@unique`, and `changes` must contain at least one known model field. The `@id` field cannot be changed.

The update is parameterized and returns `Promise<Result<User?, DbError>>`; await it before using the `Result`. On success, it returns the updated `User`, or an absent value when no record matched. Database constraint and connection failures are recoverable `DbError` values. The unique filter requirement prevents the operation from matching multiple records.

This fragment assumes an imported `User` model with `id` marked `@id` and an `email` field.

<!-- @include: ../../.fragments/modules/db/update-one/example.md -->

Use [`Model.updateMany(filter, changes)`](./update-many.md) when the filter may match multiple records.
