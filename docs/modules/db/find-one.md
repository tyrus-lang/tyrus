---
title: findOne
description: Find one model record with a checked, parameterized filter.
---

# Model.findOne

Call `User.findOne(filter)` on an imported model. It uses the same filter fields and operators as [`findMany`](./find-many.md), binds every value as a parameter, and checks the generated query against the local SQLite schema snapshot.

Its return type is `Promise<Result<Optional<User, void>, DbError>>`. Await the call to receive the `Result`, which carries database errors. On success, the inner `Optional` contains the matching record or is absent when no record matches. Narrow it with `if (user)` or provide a fallback with `??`.

The query uses `LIMIT 1`. When the filter can match multiple rows, the selected row is not deterministic unless the filter identifies a unique row, such as one using `@id` or `@unique`.

This fragment assumes `User` is an imported `@model` interface with an `email` field.

<!-- @include: ../../.fragments/modules/db/find-one/example.md -->

Use [`findMany`](./find-many.md) when the result should be a list, including the empty-list case. Database failures remain recoverable `DbError` values inside the awaited `Result`.
