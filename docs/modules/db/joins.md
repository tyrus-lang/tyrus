---
title: SQL joins
description: Inner and outer joins with checked optional result fields.
---

# SQL joins

`db.query<T>` supports `JOIN`, `INNER JOIN`, `LEFT JOIN`, `RIGHT JOIN`, and `FULL JOIN`. `LEFT OUTER JOIN`, `RIGHT OUTER JOIN`, and `FULL OUTER JOIN` are also accepted. Each `ON` clause compares one column from the newly joined table with one from a previous table using `=`. Qualify a column when its name appears in more than one table.

| Join | Columns that may be absent from a result row |
| --- | --- |
| `JOIN` or `INNER JOIN` | Newly joined table remains present; previous tables retain their earlier nullability |
| `LEFT JOIN` | Newly joined table |
| `RIGHT JOIN` | Previous tables |
| `FULL JOIN` | Previous tables and newly joined table |

Declare a field from a side that may be absent as `T?`, even when that column is nonnullable in the table schema. With several joins, a later `RIGHT JOIN` or `FULL JOIN` can make earlier tables optional. A schema nullable column also requires `T?`. The checker does not infer a nonnull result from a `WHERE` predicate, so retain the optional field type when filtering joined rows.

## Example

This example assumes the local schema snapshot contains `db_examples(id: i64, label: string)` and `db_example_roles(id: i64, title: string)`.

<!-- @include: ../../.fragments/modules/db/joins/example.md -->

`SELECT *` is accepted only when every requested result field maps to a unique column; use explicit projections and aliases when tables share names. `CROSS JOIN` and arbitrary expressions in `ON` are not supported yet. See [SQL safety](./security.md) for binding rules.
