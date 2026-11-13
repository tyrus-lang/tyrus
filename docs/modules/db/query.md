---
title: query
description: Execute typed relational queries and parameterized mutations.
---

# db.query

<SignatureBlock label="Signature">query&lt;T&gt;(sql: string, parameters: unknown[]): Promise&lt;Result&lt;T[], DbError&gt;&gt;</SignatureBlock>

<SignatureBlock label="Signature">query(sql: string, parameters: unknown[]): Promise&lt;Result&lt;void, DbError&gt;&gt;</SignatureBlock>

`db.query<T>` returns `Promise<Result<T[], DbError>>` for validated `SELECT` statements and decodes each returned row into the explicitly named interface `T`. Await every database call; a query with no matching rows resolves to an empty array inside `Ok`. For `INSERT`, `UPDATE`, and `DELETE`, call `db.query` without a type argument; a successful mutation resolves to `void` inside `Ok`.

An `Instant` passed as a SQL parameter is bound using the selected backend's representation. SQLite stores UTC text, PostgreSQL uses `TIMESTAMPTZ` and rejects sub-microsecond values, and MySQL uses `DATETIME(3)` and rejects sub-millisecond values. A typed `SELECT *` decodes Model `Instant` fields into their Tyrus representation.

`COUNT`, `SUM`, `AVG`, `MIN`, and `MAX` are supported in `SELECT`. See [SQL aggregates](./aggregates.md) for result types, aliases, examples, and empty-result behavior.

Inside a `db.query` string, the Language Server completes aggregate expressions, SQL clauses, tables, and columns from the local schema snapshot. Hover over a table or column to see its schema, or over an aggregate name to see its signature and result behavior.

To inspect a query in the editor, place the cursor inside its SQL string, open Code Actions, and choose **Explain SQL query**. The result includes normalized SQL, parameter types with values hidden, and result fields; it validates against the local snapshot without opening a database connection. Editor explanations accept SQL strings up to 64 KiB.

Inner and outer joins require result fields to reflect which side may be absent. See [SQL joins](./joins.md).

<!-- @include: ../../.fragments/modules/db/query/example.md -->

The compiler validates the statement kind, selected columns, aliases, parameter count, and parameter types against the local schema snapshot. It rejects a parameter array with more than 999 entries using `E3305`; the runtime enforces the same limit and returns `ERR_DB_PARAMETER_LIMIT` as a final guard. SQL must be a string literal and values must be supplied in a separate array literal. See [SQL safety](./security.md) for the supported statement subset and binding rules.
