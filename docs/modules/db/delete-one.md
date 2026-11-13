---
title: deleteOne
description: Delete one model record selected by a unique field.
---

# Model.deleteOne

Call `User.deleteOne(filter)` on an imported model. The filter must contain an equality condition for a field marked `@id` or `@unique`; this requirement prevents the operation from matching multiple records.

The delete is parameterized and returns `Promise<Result<User?, DbError>>`; await it before using the `Result`. On success, it returns the deleted `User`, or an absent value when no record matched. Database constraint and connection failures are recoverable `DbError` values.

This fragment assumes an imported `User` model whose `id` field is marked `@id`.

<!-- @include: ../../.fragments/modules/db/delete-one/example.md -->

Use [`Model.deleteMany(filter)`](./delete-many.md) when the filter may match multiple records.
