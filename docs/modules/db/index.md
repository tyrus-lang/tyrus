---
title: Database
description: SQL and ORM/ODM for SQLite, PostgreSQL, and MySQL with offline schema snapshots.
---

# tyrus:db

`tyrus:db` provides relational SQL statements for SQLite, PostgreSQL, and MySQL with offline schema validation. Every SQL and ORM/ODM operation is asynchronous: use `await` and handle its `Promise<Result<..., DbError>>`. Configure `database` in `tyrus.json` and provide a local schema snapshot before building. Schema introspection, drift checks, and read-only Model-to-snapshot comparison support all three SQL backends. SQLite supports safe incremental migrations, including reviewed table rebuilds. PostgreSQL and MySQL generate initial schemas and safe incremental migrations for new Model tables, additive columns, literal defaults, unique constraints, and optional foreign keys when the live schema and snapshot confirm compatibility; destructive transformations and ambiguous changes require manually authored SQL migrations. See [Database schema commands](/tooling/cli#database-schema-commands).

## Available API

- [`db.query<T>`](./query.md) executes a validated `SELECT` and returns `Promise<Result<T[], DbError>>`; SQL mutations use the same function and return `Promise<Result<void, DbError>>`.
- [`Model.findMany(filter)`](./find-many.md)
- [`Model.findOne(filter)`](./find-one.md) returns one matching record or a typed absent value using a typed, parameterized filter.
- [`Model.insertOne(data)`](./insert-one.md) inserts one model record and returns `Promise<Result<Model, DbError>>`.
- [`Model.insertMany(rows)`](./insert-many.md) inserts a bounded batch and returns `Promise<Result<Model[], DbError>>`.
- [`Model.updateOne(filter, changes)`](./update-one.md) updates a row identified by a unique field.
- [`Model.updateMany(filter, changes)`](./update-many.md) updates every row matching a non-empty filter.
- [`Model.deleteOne(filter)`](./delete-one.md) deletes a row identified by a unique field.
- [`Model.deleteMany(filter)`](./delete-many.md) deletes every row matching a non-empty filter.
- [`Model.count(filter)`](./count.md) returns the number of matching records as `Promise<Result<u64, DbError>>` and accepts `{}` to count all records.
- [`Page<T>`](./page.md) documents the explicitly imported pagination result type.
- [`ObjectId`](./models.md#objectid), [`Uuid`](./models.md#uuid), and [`Uuid7`](./models.md#uuid7) are public Model field types for MongoDB and relational ID strategies.
- [Database models](./models.md) declare and validate explicit relational table and column mappings.
- [Migrations](./migrations.md) explain how to generate, review, and apply schema migrations and seeds.
- [Database transactions](./transactions.md) document explicit transactions for `db.query` on SQL backends.
- [SQL joins](./joins.md) describe inner and outer joins and the optional result fields they require.
- [Aggregates](./aggregates.md) describes aggregate projections, result types, and empty-result behavior.
- [`DbError`](./errors.md) contains a stable error code and readable message.
- [SQL safety](./security.md) describes parameter binding, literal-only SQL, and the supported query subset.
- [API Flow](/modules/http/api-flow) connects a User route to its controller, service, repository, and database operations in one SQLite-backed example.

## Embedded database and server backends

Tyrus supports SQLite, PostgreSQL, and MySQL for runtime SQL and ORM operations through the same model API. MongoDB supports Model `count`, `findOne`, numbered and cursor-paginated `findMany`, and all six Model mutations; relationship includes and SQL queries are unavailable for MongoDB. MongoDB pagination orders by `_id`, and its count and item reads are separate operations, so concurrent writes may affect consistency between them. The SQLite engine is bundled into native Tyrus applications, so developers and application users do not need to install SQLite separately or run a database server. The application reads and writes a separate SQLite database file, so its directory must be writable and the file must be included in the application's backup plan. [SQLite overview](https://www.sqlite.org/about.html)

When `database.backend` is `mongodb`, SQL statements passed to `db.query` are rejected during compilation with `E3306`. MongoDB setup reads `database.connectionEnv` only at runtime, verifies the connection with a `ping`, and returns redacted `DbError` values for connection failures. Its pool applies `maxConnections`, `connectTimeoutMs`, `acquireTimeoutMs`, `queryTimeoutMs`, and `idleTimeoutMs`; TLS is controlled by the connection URI and is not required by Tyrus. Use `database.tls.caFile` for a project-relative private CA certificate or configure both `database.tls.certFile` and `database.tls.keyFile` for mutual TLS; when these files are configured, certificate and hostname verification remain enabled. The MongoDB pool accepts `maxConnections` (1–100), `connectTimeoutMs`, `acquireTimeoutMs`, and `queryTimeoutMs` (1–120,000 ms), and `idleTimeoutMs` (1–86,400,000 ms). `acquireTimeoutMs` limits how long an operation waits for a Tyrus runtime connection slot and returns `ERR_DB_POOL_TIMEOUT` on expiry. `maxLifetimeMs` is not supported and is rejected when configured. Read filters support equality, comparison, membership, and `or`; `like` uses a case-sensitive pattern where `%` and `_` are wildcards. Filter and mutation values are checked against their declared Model field types during compilation. MongoDB Model fields may contain nested objects and arrays; Tyrus converts these values to BSON and limits nesting to 100 levels, returning `ERR_DB_VALUE_TOO_DEEP` when exceeded. Runtime rejects a BSON document or filter larger than MongoDB's 16 MiB document limit with `ERR_DB_DOCUMENT_TOO_LARGE` ([MongoDB document limits](https://www.mongodb.com/docs/manual/reference/limits/)). Tyrus does not translate SQL into document queries. Relationship includes on non-SQLite backends are rejected during compilation with `E3307`.

The runtime processes at most 32 database requests concurrently and accepts at most 128 additional queued requests. When that queue is full, the call returns `ERR_DB_BUSY` so applications can retry with backoff instead of accumulating unbounded pending work.

PostgreSQL and MySQL servers remain external to the application. Configure `database.backend` as `postgresql` or `mysql` and set `database.connectionEnv` to the name of the environment variable containing the connection URL; `urlEnv` is not accepted as an alias. Tyrus reads that variable at runtime from the process environment or the file configured by `security.envFile`; the URL is not embedded in the generated program or written to diagnostics. The optional `database.pool` block accepts `maxConnections` (1–100), `connectTimeoutMs`, `acquireTimeoutMs` and `queryTimeoutMs` (1–120,000 ms each), and `idleTimeoutMs` and `maxLifetimeMs` (1–86,400,000 ms each). Defaults are 10 connections, 10,000 ms, 5,000 ms, 30,000 ms, 30,000 ms and 1,800,000 ms, respectively. Connection, acquisition and operation timeouts return `ERR_DB_CONNECTION_TIMEOUT`, `ERR_DB_POOL_TIMEOUT` and `ERR_DB_QUERY_TIMEOUT`, respectively. SQLite rejects this block. MongoDB accepts `maxConnections`, `connectTimeoutMs`, `acquireTimeoutMs`, `queryTimeoutMs`, and `idleTimeoutMs` with the documented ranges; unsupported MongoDB options are rejected instead of silently ignored. Database TLS is optional for PostgreSQL, MySQL, and MongoDB. Without `database.tls` files, Tyrus leaves TLS behavior to the connection URL and database driver, so local development servers can use plaintext and production URLs can opt into TLS. If `database.tls` configures a CA or client identity, Tyrus enables verified TLS; for PostgreSQL this uses `sslmode=verify-full`, and for MySQL it uses `ssl-mode=VERIFY_IDENTITY`. A weaker mode is rejected with `ERR_DB_TLS_CONFIGURATION` when TLS files are configured. Set `database.tls.caFile` to a project-relative CA certificate path, or configure both `database.tls.certFile` and `database.tls.keyFile` for mutual TLS. MongoDB supports the same CA and client identity settings. Tyrus confines TLS file paths configured in `tyrus.json` to the project. SQLite is an embedded local database and does not use network TLS.

```json
{
  "database": {
    "backend": "postgresql",
    "connectionEnv": "DATABASE_URL",
    "tls": {
      "caFile": "certs/database-ca.pem",
      "certFile": "certs/database-client.pem",
      "keyFile": "certs/database-client-key.pem"
    }
  }
}
```

PostgreSQL currently supports booleans, signed integers, floating-point numbers, text, bytes, and native UUID fields for Model identifiers. MySQL currently supports `SMALLINT`, `INT`, `BIGINT`, floating-point numbers, text, bytes, and UUID identifiers stored as `CHAR(36)`; MySQL `TINYINT` and `BOOLEAN` result columns are not supported yet. Relation includes are currently SQLite-only; PostgreSQL and MySQL return `ERR_DB_FEATURE_UNSUPPORTED` for them. MySQL supports full-range `u64` Model fields, including `uuid_short`; PostgreSQL cannot bind `u64` values above the signed 64-bit range. Backend-native types outside these mappings return a recoverable `DbError`. PostgreSQL/MySQL support initial Model migrations, new Model tables, additive columns, literal defaults, unique constraints, optional foreign keys under validated conditions, and backend-specific manual SQL files. Incremental generation verifies the live schema against the snapshot and rejects destructive, definition-changing, or ambiguous cases. Migration status, application, schema introspection, drift checks, and read-only Model-to-snapshot comparison work for all three SQL backends; destructive or in-place Model migration generation remains SQLite-only. Builds and the Language Server remain offline and use the configured local snapshot. Recoverable failures use `Result`, never `throws`.

## Backend capability matrix

The runtime maps database values to Tyrus scalars rather than exposing driver types. Use `i64`, `u64`, and `f64` for numeric data; PostgreSQL cannot bind `u64` values above the signed 64-bit range, while MySQL supports the full `u64` range for typed projections and bound values; `f64` does not preserve exact decimal arithmetic. Model `Instant` fields map to UTC text in SQLite, `TIMESTAMPTZ` in PostgreSQL, and `DATETIME(3)` in MySQL; values exceeding a backend's stored precision are rejected rather than rounded. MongoDB stores `Instant` as BSON Date with millisecond precision and preserves civil temporal values as BSON documents; an `Instant` with sub-millisecond precision is rejected rather than rounded.

| Capability | SQLite | PostgreSQL | MySQL | MongoDB |
| --- | --- | --- | --- | --- |
| Bound parameter syntax | `?` | Tyrus `?`, translated to `$1`, `$2`, and so on | `?` | Model values use separate parameters encoded as BSON |
| Binary values | SQLite `BLOB` ↔ `u8[]` | PostgreSQL `bytea` ↔ `u8[]` | MySQL `BLOB` ↔ `u8[]` | BSON Binary ↔ `u8[]` |
| Structured values | Not supported as SQL parameters | Not supported as SQL parameters | Not supported as SQL parameters | Nested objects and arrays map to BSON documents and arrays, limited to 100 levels and 16 MiB per document |
| Date and time values | Model `Instant` maps to UTC text | Model `Instant` maps to `TIMESTAMPTZ` at microsecond precision | Model `Instant` maps to `DATETIME(3)` at millisecond precision | `Instant` maps to BSON Date at millisecond precision; civil temporal values map to BSON documents |
| Quoted identifiers | Double quotes; matching follows SQLite's case-insensitive identifier rules | Double quotes; quoted names preserve case | Tyrus double-quoted identifiers are translated to backticks; table-name case behavior depends on server configuration | Model collection and field names come from validated model metadata |
| Batch inserts | At most 500 rows and 999 bound parameters | At most 500 rows and 999 bound parameters | At most 500 rows and 999 bound parameters | At most 500 documents, each at most 16 MiB; SQL parameter limits do not apply |
| Batch execution | One multi-row statement; the backend applies its statement atomicity | One multi-row statement; the backend applies its statement atomicity | One multi-row statement; the backend applies its statement atomicity | One document batch operation; partial progress is reported as a recoverable operation failure |
| Mutation result | Inserts return resources, single-row updates and deletes return an optional resource, and multi-row updates and deletes return an affected-row count | Inserts return resources, single-row updates and deletes return an optional resource, and multi-row updates and deletes return an affected-row count | Inserts return resources, single-row updates and deletes return an optional resource, and multi-row updates and deletes return an affected-row count | Inserts return documents, single-row updates and deletes return an optional document, and multi-row updates and deletes return an affected-document count |
| Numbered and cursor pagination | Supported; ordering uses `@id`, and `total` is read with the page in one transaction | Supported; ordering uses `@id`, and `total` is read with the page in one `REPEATABLE READ` transaction | Supported; ordering uses `@id`, and `total` is read with the page in one `REPEATABLE READ` transaction | Supported on `_id`; count and items are separate reads and may observe concurrent changes |
| Public transaction API | Explicit handles for `db.query`; ORM context is not available | Explicit handles for `db.query`; ORM context is not available | Explicit handles for `db.query`; ORM context is not available | Not available |
| Relation includes | Supported, with explicit collection limits | `E3307` during compilation; the runtime guard returns `ERR_DB_FEATURE_UNSUPPORTED` | `E3307` during compilation; the runtime guard returns `ERR_DB_FEATURE_UNSUPPORTED` | Not supported; `E3307` during compilation |
| Schema introspection and migration commands | Introspection, generation, status and apply | Initial Model generation, new tables, safe additive columns, literal defaults, unique constraints, optional foreign keys after compatibility checks, manual SQL files, introspection, status and apply | Initial Model generation, new tables, safe additive columns, literal defaults, unique constraints, optional foreign keys after compatibility checks, manual SQL files, introspection, status and apply | Creates missing collections with Model-derived validators on first insert; existing collections are unchanged; no introspection or migration commands |

The SQL database drivers return `DbError` for native result types outside the supported boolean, integer, floating-point, text, and binary mappings. `db.query` uses `?` placeholders in source on every SQL backend; the runtime rewrites only the PostgreSQL form. SQL backends enforce the same conservative 999-parameter statement limit and 500-row batch ceiling. MongoDB has no SQL query path and applies its own BSON, nesting, and 500-document batch limits.
