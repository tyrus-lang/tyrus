---
title: findMany
description: Query model records with checked, parameterized filters.
---

# Model.findMany

Import an exported `@model` interface and call `Model.findMany(filter)` to retrieve its records. The method returns `Promise<Result<Model[], DbError>>` and applies the model's physical table and column mappings.

Await the method before using its `Result`. Flat filter fields combine with `AND`. A scalar value means equality, as in `User.findMany({ active: true })`. For comparisons, use a nested object with one or more of `eq`, `ne`, `gt`, `gte`, `lt`, and `lte`, such as `User.findMany({ age: { gte: 18 } })`.

The comparison operators mean: `eq` is equal to, `ne` is not equal to, `gt` is greater than, `gte` is greater than or equal to, `lt` is less than, and `lte` is less than or equal to. When a field has more than one operator, all of its conditions must match.

Use `or` with a nonempty array of filter objects to match any alternative. Fields in each object combine with `AND`, the objects combine with `OR`, and fields beside `or` still combine with the whole group using `AND`.

Use `in` with a nonempty array to match any listed value. In a `like` pattern, `%` matches zero or more characters and `_` matches one character: `"Ada"` matches exactly `Ada`, `"Ada%"` matches values starting with `Ada`, `"%son"` matches values ending with `son`, and `"%Ada%"` matches values containing `Ada` anywhere. Matching follows the configured backend’s `LIKE` behavior. All values remain parameters and are checked against the selected model column.

This example assumes the model declares `active`, `age`, and `role` fields.

<!-- @include: ../../.fragments/modules/db/find-many/or-filter.md -->

## Relationship loading

Declare model relationships with `@belongsTo("foreignKeyField")`, `@hasOne("foreignKeyField")`, `@hasMany("foreignKeyField")`, or `@manyToMany("pivotTable", "sourceKey", "targetKey")`, then pass their names in `{ include: ["team"] }`. Select a subset of scalar fields with `{ include: [{ team: ["id", "name"] }] }`; the result type reflects the selected fields. Unprojected includes return the full related Model. Relationship loading is explicit, uses one SQL statement, and supports one level only. Including a `hasMany` or `manyToMany` relation requires `relationLimit` from 1 through 500, which caps each included collection. Related collections are ordered by the related Model's `@id` for stable results.

<!-- @include: ../../.fragments/modules/db/find-many/relationships.md -->

## Numbered pagination

Pass `{ paginate: true, pageSize, page? }` as the second argument to return a numbered page. The default page is 1. `Page<T>` contains `items`, `total`, `pageSize`, `page`, and `totalPages`; `total` counts all records matching the filter before pagination. MongoDB supports numbered pagination by sorting on `_id` and applying the requested offset. Import `Page` from `tyrus:db` when naming this result in a function signature.

<!-- @include: ../../.fragments/modules/db/find-many/pagination.md -->

## Cursor pagination

Set `cursor: true` to use cursor pagination. Omit `after` on the first request; for the next request, pass the previous page's `nextCursor` unchanged. The cursor is the serialized `@id` value of the last returned row, not an item count or page number. For an integer `@id`, `after: "10"` continues after id 10, not at the tenth result. Results are ordered by the model's `@id`, and `hasMore` indicates whether another page exists. `total` still counts all matching records, independent of the cursor position. MongoDB uses the generated `_id` for ordering and cursor comparison, so its cursor token is the returned serialized 24-character ObjectId value.

The compiler checks each field against the model and validates the generated query and parameter types against the local schema snapshot for the configured relational backend. MongoDB uses the same model filter API but does not use a relational schema snapshot. Its page count and items are read separately, so concurrent writes can change the observed total between page requests. Values bind through placeholders; table and column identifiers come only from the model declaration.

Model mutations are documented on the [Models](./models.md) page. Use [`findOne`](./find-one.md) to return one record or a typed absent value.
