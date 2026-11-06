---
title: SQL aggregates
description: COUNT, SUM, AVG, MIN, and MAX in typed database queries.
---

# SQL aggregates

Use aggregate expressions in the `SELECT` list of [`db.query<T>`](./query.md). Declare each result field in `T` with the type shown below, and give every expression an explicit `AS` alias matching that field.

| Expression | Argument | Result field |
| --- | --- | --- |
| `COUNT(*) AS total` | All matching rows | `total: i64` |
| `COUNT(column) AS total` | Nonempty values in the column | `total: i64` |
| `SUM(column) AS sum` | `i64` or `f64` column | `sum: i64?` or `sum: f64?` |
| `AVG(column) AS average` | `i64` or `f64` column | `average: f64?` |
| `MIN(column) AS minimum` | `i64`, `f64`, or `string` column | Optional column type |
| `MAX(column) AS maximum` | `i64`, `f64`, or `string` column | Optional column type |

`COUNT(*)` counts rows; `COUNT(column)` counts rows with a value in that column. Both return `0` when no values match. The other aggregates return an empty optional in that case, so use `??` or narrow the value before using it. An aggregate query without `GROUP BY` returns one row even when no source rows match.

## Example

This example assumes the local schema snapshot contains `db_examples(id: i64)` and the runtime database has the same table.

<!-- @include: ../../.fragments/modules/db/aggregates/example.md -->

You can combine multiple aggregates in one `SELECT`.

## GROUP BY

Use `GROUP BY` to combine plain columns and aggregates. Every selected plain column must appear in the grouping list, either by its column name, qualified column name, or explicit `SELECT` alias. `SELECT *` is not accepted with `GROUP BY`; list result columns explicitly. `HAVING` filters groups after aggregation and may refer to grouped columns or aggregate aliases. Bind values through `?` parameters. `ORDER BY` may refer to a grouped column or an aggregate alias. The result contains one row per group, or an empty array if no groups match.

This example returns one row per `id` value:

<!-- @include: ../../.fragments/modules/db/aggregates/group-by.md -->

SQL must be a string literal, and dynamic values belong in the separate parameter array.
