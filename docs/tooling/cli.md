---
title: CLI
description: tyrus command line interface for scaffolding, checking, building, running, formatting, linting, and editor integration.
---

# CLI

The Tyrus CLI builds, runs, formats, analyzes, and scaffolds Tyrus projects.

## Create a Project

<!-- @include: ../.fragments/tooling/cli-create-project.md -->

Run `tyrus new` without arguments. The CLI asks for the project name and then the project target.

<!-- @include: ../.fragments/tooling/cli-target-flags.md -->

For an API target, the CLI asks whether to configure no database, SQLite, PostgreSQL, MySQL or MongoDB. Choosing SQLite writes the recommended `database.sqlite` settings and creates the `data/` directory. Server backends use `DATABASE_URL` as `connectionEnv`; the existing `.env` and `.env.example` scaffolding includes that variable.

`tyrus new` also writes `__tests__/` with one passing test, so `tyrus test` reports a result from the first run.

::: tip API projects
Choosing the API target in the `tyrus new` prompts creates API-oriented structure and lint defaults for route, controller, service, and repository layers. The CLI also asks which database backend to configure; choosing no database leaves database setup out of `tyrus.json`.
:::

## Common Commands

<!-- @include: ../.fragments/tooling/cli-common-commands.md -->

## Project Commands

| Command | Purpose |
|---------|---------|
| `tyrus new` | Creates a project through prompts for its name and target. |
| `tyrus --init` | Adds missing default keys to `tyrus.json` while keeping existing values. |
| `tyrus --default` | Resets `tyrus.json` to generated project defaults, removing custom scripts and settings. |
| `tyrus info` | Prints project metadata. |
| `tyrus clean [--cache] [--dry-run] [--force]` | Removes the project `target/` directory and optionally the global Tyrus cache. |
| `tyrus check` | Parses, type-checks, and borrow-checks without generating a native binary. |
| `tyrus build [--release] [--rotate-seal-key] [--pgo-generate\|--pgo-use]` | Compiles the project. |
| `tyrus pgo-merge` | Fuses the profiles an instrumented run wrote into one file. |
| `tyrus run [<path>\|<script>] [--release] [--rotate-seal-key] [-- <args>...]` | Runs a named `tyrus.json` script, or compiles and runs `src/main.tyr` or a standalone source file; arguments after `--` are passed to the script or program. |
| `tyrus test [<path>]` | Runs every `*.test.tyr` file and reports the results. |
| `tyrus watch [--run\|--test\|--command <cmd>] [--debounce <ms>] [--clear]` | Builds again whenever a `.tyr` file or `tyrus.json` changes; `--run` starts the program after each successful build, `--test` runs the tests, and `--command` runs a shell command instead. |
| `tyrus dev [--debounce <ms>] [--clear]` | Rebuilds and restarts the project after every source change using the same watcher and graceful process replacement as `tyrus watch --run`. |
| `tyrus db introspect` | Reads the configured SQLite, PostgreSQL, or MySQL database in read-only mode and atomically writes `tyrus-db-schema.json`. |
| `tyrus db check` | Compares the live SQLite, PostgreSQL, or MySQL schema with the snapshot without changing the database. |
| `tyrus db diff` | Shows differences between the schema snapshot and the live SQL database without writing either schema; all SQL backends also compare project Models with the snapshot. |
| `tyrus db explain-sql [SQL]` | Validates one SQL statement against the local snapshot and prints normalized SQL, inferred parameter types, and result fields with values hidden. Pass SQL as a quoted argument or through stdin. |
| `tyrus db migrate create <name>` | Creates the next numbered SQL migration file with a backend-specific starter comment for SQLite, PostgreSQL, or MySQL. |
| `tyrus db migrate generate <name>` | Generates SQLite migrations from `@model` declarations, or an initial PostgreSQL/MySQL migration from supported Model fields. Review the SQL before applying it. |
| `tyrus db migrate status` | Lists applied and pending migrations for SQLite, PostgreSQL, or MySQL without changing the database. |
| `tyrus db migrate apply` | Applies pending migrations with a SQLite/PostgreSQL transaction or a MySQL recovery marker, and records checksums. |
| `tyrus db seed status [name]` | Lists applied and pending Tyrus seed programs without changing the database. |
| `tyrus db apply <name.tyr>` | Runs one seed program from `db/seeds/` and records its checksum. |

`tyrus clean` removes only the project `target/` directory. It asks for confirmation in an interactive terminal; use `--force` to skip the prompt, and non-interactive runs require that flag before deleting anything. Add `--dry-run` to estimate the space that would be freed without changing files. Add `--cache` to include `~/.tyrus/cache/`; Tyrus warns that this cache is shared across projects and asks for confirmation separately.

Pass a source path to `tyrus run` to execute a separate file without changing `src/main.tyr`. This is useful for quick experiments and examples that contain top-level statements:

<!-- @include: ../.fragments/tooling/cli/project-commands.md -->

Without a path, `tyrus run` continues to compile and execute `src/main.tyr`.

Generated projects include `fmt`, `lint`, and `test` entries under `scripts` in `tyrus.json`. Declare additional project scripts as string values and run one by name with `tyrus run <name>`, such as `tyrus run example`; arguments for the configured command follow `--`. `tyrus --init` preserves custom script entries while adding missing defaults, and `tyrus --default` replaces the manifest with generated project defaults.

`tyrus watch` observes `src/` (and `tests/` with `--test`) and waits `--debounce` milliseconds, 300 by default, so a save that touches several files starts one cycle. A failed build prints its diagnostics and keeps watching. With `--run`, the program from the previous cycle is stopped before the next build, so a server releases its port. `target/` and `.git` are always ignored; list further paths in a `.tyrusignore` file at the project root, one per line, where `src/generated` ignores that directory and a bare name such as `fixtures` ignores any directory with that name. Status lines go to stderr, so the output of the program or command stays on stdout.

`tyrus dev` is the focused development shortcut for `tyrus watch --run`: it accepts only `--debounce` and `--clear`, rebuilds after changes under `src/` or `tyrus.json`, stops the previous program before starting the replacement, and keeps watcher status on stderr.

## Database Schema Commands

Configure a project-relative SQLite path in `tyrus.json`. The configuration has no credential fields, and database and snapshot paths must resolve inside the project directory.

<!-- @include: ../.fragments/tooling/cli/db-config.md -->

For SQLite, `database.sqlite.journalMode` accepts `wal` or `delete`, `synchronous` accepts `full` or `normal`, and `busyTimeoutMs` accepts values from 1 through 120000. Defaults are WAL, FULL and 5000 ms; `:memory:` always uses SQLite's memory journal. `database.sqlite` is rejected for other backends. WAL requires compatible shared-memory and locking support and does not work on network filesystems; Tyrus confirms SQLite applied the requested mode but cannot verify filesystem locking or storage flush guarantees. In WAL mode, FULL synchronizes each transaction commit, while NORMAL synchronizes less often and can lose recent commits after power loss while preserving database consistency. SQLite checkpoints automatically during normal operation and when the last connection closes. Do not copy only the main database file while it is open; use a consistent SQLite backup operation. See the [SQLite WAL documentation](https://www.sqlite.org/wal.html) and [synchronous pragma documentation](https://www.sqlite.org/pragma.html#pragma_synchronous) for the storage guarantees and trade-offs.

Run `tyrus db introspect` when the database schema changes to create or refresh the versioned snapshot. The command supports SQLite, PostgreSQL, and MySQL and opens the configured database in read-only mode. Builds and the Language Server remain offline; they do not connect to the database. Run `tyrus db check` in CI or before a build to detect a missing or stale snapshot. `tyrus db diff` compares the live database with the snapshot and compares project Models with the snapshot for SQLite, PostgreSQL, and MySQL.

Use `tyrus db diff` to review differences between project Models, the local snapshot, and the current database before changing any of them. It reports Model table and column changes, SQLite schema version changes, logical and native column type, nullability, default-expression, and table-engine differences, and changes to primary keys, unique constraints, foreign keys, and indexes. Model comparison is read-only; destructive or in-place Model migration generation remains SQLite-only. `tyrus db check` remains the CI command that fails when database drift exists.

Pass one quoted SQL statement to `tyrus db explain-sql` or pipe it through stdin to inspect normalized SQL, inferred parameter types, and result fields without opening a database connection. For `SELECT`, each result field includes its type and optionality; mutations show `void`. Literal string and numeric values are redacted in the displayed SQL, and bound parameter values are never displayed.

<!-- @include: ../.fragments/tooling/cli/db-explain.md -->

## SQLite Migrations

Create a migration, edit its SQL, inspect the pending list, and apply it explicitly. Use `tyrus db migrate generate <name>` to create initial `CREATE TABLE` statements when the configured database does not exist. For an existing database, it can create tables for new Models, safe additive `ALTER TABLE ... ADD COLUMN` statements for nullable fields or fields with literal defaults, and rebuild a table when a Model removes columns or changes its primary key or `@unique` constraints. Rebuilds copy retained columns and restore custom index definitions from SQLite's schema catalog. They are refused if a removed column is used by an index, or if foreign keys, views, triggers, or simultaneous additions prevent safe preservation. Applying a changed key constraint can fail if existing rows violate it; SQLite rolls back the transaction. Incremental generation requires a current schema snapshot. It rejects schema drift and changes to existing column definitions or foreign-key constraints; write those migrations manually. Types without a lossless SQLite mapping are rejected. Generation creates a reviewable file and never changes the database. Refresh the schema snapshot after applying a schema migration.

<!-- @include: ../.fragments/tooling/cli/db-migrate.md -->

For PostgreSQL and MySQL, `tyrus db migrate generate <name>` writes initial schemas without connecting to the server. After that, it connects to the configured server, verifies the active schema against its snapshot, and generates new Model tables or safe additive columns; new unique columns must be optional and have no default, and PostgreSQL and MySQL can add nullable foreign-key columns when the referenced table, unique key, and compatible type exist in the snapshot; MySQL also requires InnoDB on both tables and matching signed integer types recorded by introspection. Other key changes, changes to existing definitions, and destructive operations require manually authored SQL. Added columns must be nullable or have a literal default, and Models with existing defaults or foreign-key metadata are conservatively rejected for incremental generation. It refuses `@timestamps` for existing Models and unsupported field types. `status` and `apply` also support both server backends; PostgreSQL uses a transaction and advisory lock, while MySQL records a recovery marker because DDL may commit implicitly.

Migration files use names such as `0001_add_users.sql`. Each file can contain multiple SQLite statements. The CLI records a SHA-256 checksum in `_tyrus_migrations`, rejects changed or missing applied files and pending versions older than an applied version, and serializes concurrent application with a SQLite write lock. A failed migration rolls back the whole pending batch. `status` opens an existing database read-only; neither `status` nor a build applies migrations. Files must be regular UTF-8 files no larger than 4 MiB. Managed migrations do not accept transaction commands, `PRAGMA`, `ATTACH`, `DETACH`, or `VACUUM`; keep transaction control with the CLI. Migrations require a persistent SQLite path and have no automatic downgrade.

Create seed programs manually as `.tyr` files under `db/seeds/`. Seed history is supported for SQLite, PostgreSQL, and MySQL; MongoDB does not yet support CLI-managed seed history. A seed can use the Model API or `db.query`; run one file with `tyrus db apply default_roles.tyr` after applying migrations. `tyrus db seed status [name]` lists seed states. The CLI records a SHA-256 checksum in `_tyrus_seeds` and refuses to run an applied file again or accept changed or missing applied files. The seed program runs as a separate Tyrus process, so the CLI cannot wrap its arbitrary operations in one database transaction; use the database transaction API when several changes must be atomic. A failed or interrupted program remains marked `started` and blocks retries. Inspect the database, repair or undo partial changes, then remove that seed's `_tyrus_seeds` row before retrying; create a new seed file for later data changes.

## Tooling Commands

| Command | Purpose |
|---------|---------|
| `tyrus fmt [--check] [--stdout] [--init] [--default] [<path>...]` | Formats source files. |
| `tyrus lint [--fix] [--format json] [--max-warnings <n>] [--explain <rule>] [--init] [--default] [<path>...]` | Reports lint diagnostics. |
| `tyrus lsp` | Starts the Language Server over stdio. |
| `tyrus mcp install` | Downloads Tyrus documentation and automatically configures detected MCP clients. |
| `tyrus mcp update` | Refreshes the MCP documentation cache and managed client entries. |
| `tyrus mcp status` | Shows MCP cache and client configuration status. |
| `tyrus mcp uninstall` | Removes unchanged Tyrus MCP entries and the local documentation cache. |
| `tyrus mcp serve` | Starts the MCP stdio server; configured clients run this command. |
| `tyrus repl` | Starts the interactive REPL console. |

## Output Conventions

Normal program output belongs on stdout. Compiler, linter, runtime failure, prompt, and progress output belongs on stderr so generated programs remain pipe-friendly.

::: details CI example
<!-- @include: ../.fragments/tooling/cli-ci-example.md -->
:::

### Profile-guided optimization

`-O3` guesses which branches are taken; a profile knows. Building against one lets the optimizer put hot blocks together and move cold ones, an error path, a rarely taken branch, out of the way, which is the one thing the source alone cannot say.

It is three steps, because a profile has to come from a real run:

<!-- @include: ../.fragments/tooling/cli/pgo-workflow.md -->

The gain is layout, so it shows up where there are branches to predict and not where there is arithmetic to do: measured at **1.20x** on a validation loop with five skewed branches and cold error paths, and **1.03x** on a plain arithmetic loop. Both flags require `--release`; an instrumented `-O0` build profiles code the optimizer never sees.

`llvm-profdata` has to come from the same toolchain as the compiler driver, because the raw profile carries a format version tied to it. The right one is found automatically; `TYRUS_LLVM_PROFDATA` overrides the choice.
