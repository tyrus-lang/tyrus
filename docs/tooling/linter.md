---
title: Linter
description: Complete tyrus-lint command, configuration, rule, and fix reference.
---

# Linter

`tyrus lint` reports style, correctness, safety, performance, and architecture diagnostics that are separate from compiler errors. The linter reads `tyrus-lint.json` from the project root when it exists; omitted rules use their built-in default severity.

## Commands

<!-- @include: ../.fragments/tooling/commands.md -->

`--fix` applies deterministic safe fixes and then runs lint again. `--format json` emits machine-readable reports. `--max-warnings 0` is useful for CI. `--explain <rule>` prints the rule documentation in the terminal. `--init` creates or updates `tyrus-lint.json` without changing configured severities. `--default` resets the file to built-in defaults.

## Configuration

The file is a flat JSON object that maps rule names to severities. There is no `extends`, no `presets`, and no `rules` wrapper.

<!-- @include: ../.fragments/tooling/config.md -->

| Severity | Meaning |
|----------|---------|
| `off` | Rule disabled. |
| `info` | Informational diagnostic. |
| `warn` | Warning; does not fail CI by default. |
| `error` | Error; exits with code `1`. |

## Rules

| Rule | Default | Category | Autofix | Summary |
|------|---------|----------|---------|---------|
| `api-no-verb-in-path` | `warn` | style | no | Warns when route paths use action verbs instead of resource-oriented paths. |
| `api-prefer-resource-nouns` | `warn` | style | no | Warns when route paths do not look like resource nouns. |
| `controller-must-send-response` | `error` | correctness | no | Requires every controller success path to send a response. |
| `controller-no-db-access` | `error` | safety | no | Forbids direct database access from controllers. |
| `controller-no-try-catch` | `warn` | safety | no | Warns when controllers catch expected errors instead of delegating mapping. |
| `db-explicit-select-columns` | `info` | style | yes | Prefers explicit `SELECT` projections and expands fields from the local result interface. |
| `db-no-extra-select-columns` | `info` | performance | no | Warns when a typed query selects output columns that are not declared by its result interface. |
| `exhaustive-match` | `error` | correctness | no | Requires `match` over closed types to cover every case. |
| `http-no-raw-status-code` | `warn` | safety | no | Suggests `HttpStatus` instead of raw numeric response codes. |
| `no-duplicate-case` | `error` | correctness | no | Detects duplicate `switch` or `match` cases. |
| `no-duplicate-imports` | `error` | correctness | no | Detects multiple import declarations from the same module. |
| `no-empty-block` | `warn` | style | no | Detects empty blocks without an explanatory comment. |
| `no-empty-catch` | `warn` | safety | no | Detects empty `catch` blocks without an explanatory comment. |
| `no-floating-enum` | `error` | correctness | no | Forbids `f64`/`f128` values in enums. |
| `no-force-unwrap` | `error` | safety | no | Forbids forced optional unwrap with `expr!`. |
| `no-heterogeneous-enum` | `error` | correctness | no | Forbids mixing primitive value types inside one enum. |
| `no-implicit-enum-primitive-cast` | `error` | safety | no | Requires explicit `as` casts from enum values to primitives. |
| `no-implicit-heavy-move` | `warn` | performance | no | Warns about surprising moves of large or non-Copy values. |
| `no-layer-skip` | `warn` | safety | no | Warns when API code skips route-controller-service-repository layering. |
| `no-null-literal` | `error` | correctness | no | Forbids the `null` keyword; use `T?`, optional fields, and `??`. |
| `no-shadow` | `warn` | style | no | Detects bindings that shadow enclosing bindings. |
| `no-string-concat-in-loop` | `warn` | performance | no | Detects repeated string concatenation inside loops. |
| `no-uninitialized-variable` | `warn` | correctness | no | Surfaces definite-assignment diagnostics for deferred `let x: T;`. |
| `no-unnecessary-allocation` | `info` | performance | no | Detects clearly unnecessary allocations. |
| `no-unnecessary-await` | `warn` | style | yes | Detects and fixes redundant `return await promise` delegation. |
| `no-unnecessary-clone` | `warn` | performance | no | Detects clones that could be replaced with borrows. |
| `no-upward-imports` | `error` | safety | no | Forbids lower API layers importing higher layers. |
| `no-unreachable-code` | `error` | correctness | no | Detects code after `return`, `break`, `continue`, or `throw`. |
| `no-unsafe-finally` | `error` | safety | no | Detects control-flow changes inside `finally`. |
| `no-useless-null-check` | `warn` | correctness | no | Detects null checks on values that can never be null. |
| `no-unused-functions` | `warn` | correctness | no | Detects non-exported functions that are never called. |
| `no-unused-imports` | `warn` | correctness | yes | Detects and can remove unused import specifiers. |
| `no-unused-vars` | `warn` | correctness | no | Detects unused variables; prefix with `_` when intentional. |
| `prefer-borrow` | `info` | performance | no | Suggests passing references when ownership is unnecessary. |
| `prefer-const` | `warn` | style | yes | Suggests `const` for bindings that are never reassigned. |
| `prefer-nullish-coalescing` | `info` | style | no | Suggests `??` for nullable fallback patterns. |
| `repository-no-http-error` | `warn` | safety | no | Warns when repositories depend on `HttpReply`. |
| `repository-only-db-access` | `error` | safety | no | Restricts direct database access to repository modules. |
| `route-no-business-logic` | `warn` | complexity | no | Warns when route modules contain business logic. |
| `service-no-reply` | `error` | safety | no | Forbids services from depending on `HttpReply`. |
| `use-caught-error` | `warn` | safety | no | Detects caught errors that are never used. |

## Exhaustive Match

`exhaustive-match` requires a `match` over a closed type, a tagged union, a literal union, or a boolean, to cover every case. It is enforced by the compiler as `E0004`, so it reports even with the lint rule disabled.

<!-- @include: ../.fragments/tooling/exhaustive-match.md -->

## Nullable Code

tyrus does not use the `null` literal in normal code. Express absence with `T?`, optional fields, optional parameters, and `??`.

<!-- @include: ../.fragments/tooling/nullable-code.md -->

## API Architecture

Projects created by choosing the API target in the `tyrus new` prompts receive API-oriented lint defaults. Route modules declare routes, controllers translate HTTP boundaries, services contain business behavior, and repositories own direct database access.

<!-- @include: ../.fragments/tooling/api-architecture.md -->

## Quick Fixes

Rules expose Quick Fixes when the edit is deterministic. `prefer-const` can rewrite `let` to `const`; `no-unused-imports` can remove unused import specifiers. Rules that require a design choice, such as renaming a shadowed variable, report help but do not invent an edit.

<!-- @include: ../.fragments/tooling/quick-fixes.md -->

::: warning Architecture rules are intentionally not auto-fixed
Rules such as `controller-no-db-access` and `service-no-reply` identify design problems. They require moving code to the correct layer, not a mechanical edit.
:::
