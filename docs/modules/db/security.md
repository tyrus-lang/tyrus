---
title: SQL safety
description: Parameter binding and the supported SQL surface of tyrus:db.
---

# SQL safety

`db.query<T>` for `SELECT` and `db.query` for mutations accept SQL string literals and a separate array literal for values. Each `?` placeholder is bound as a value by the configured relational database driver; parameter values are never concatenated into the SQL text.

Placeholders cannot represent table names, column names, operators, or SQL fragments. Dynamic SQL expressions and concatenation are rejected by these APIs. Choose between complete, fixed SQL literal alternatives in program logic when the application needs a small set of query shapes.

The compiler validates supported statements against the local schema snapshot. `db.query` accepts a single-table `INSERT`, `UPDATE`, or `DELETE`; `UPDATE` and `DELETE` require `WHERE`. `db.query<T>` accepts the documented `SELECT` subset, including [inner and outer joins](./joins.md) with equality comparisons in `ON`. `CROSS JOIN` is not supported yet. Qualify columns with a table name or alias when more than one joined table contains that name; ambiguous references are rejected.

[`GROUP BY`](./aggregates.md#group-by) accepts explicit grouping columns and validates every plain projection against them. `HAVING` is supported for filtering groups after aggregation and may refer to grouped columns or aggregate aliases. See the [GROUP BY example](./aggregates.md#group-by), which uses a bound parameter in `HAVING`.

For `SELECT *`, code generation replaces `*` with the quoted fields of the requested result interface, so the configured relational backend returns only the columns the decoder needs. With joins, each requested result field must map to one unambiguous column; otherwise, select and alias the columns explicitly. Explicit projections may use aliases, but their output names must uniquely and exactly match the result interface fields. Unknown columns, missing fields, extra fields, ambiguous aliases, unsupported expressions, and mismatched parameter types or counts are rejected during validation.

These restrictions and bound parameters prevent query values from changing the SQL structure through the `tyrus:db` APIs. Continue to validate authorization and business rules in application code; schema validation does not decide whether a caller is allowed to access a row.

Use [`tyrus db explain-sql`](/tooling/cli#database-schema-commands) with SQL piped through stdin to inspect normalized SQL and inferred parameter types. It validates against the local snapshot without opening the database, redacts inline string and numeric literals, and never prints bound values.
