---
title: Syntax Overview
description: A short overview of the syntax used by tyrus.
---

# Syntax Overview

tyrus uses concise, familiar syntax while preserving its language guarantees. Semantic analysis applies native checks for initialization, nullability, ownership, and control flow.

## Variables

<!-- @include: ../.fragments/docs/syntax-variables.md -->

`const` bindings are immutable after initialization. `let` bindings can be reassigned when the new value is compatible with the declared type.

## Functions

<!-- @include: ../.fragments/docs/syntax-functions.md -->

Functions use explicit parameter and return types. A non-`void` function must return a value on every reachable path.

`Result<T, E>` is a type-level success-or-error value. See [Result values](../language/types.md#result-values) for its payload rules, constructors, narrowing, and propagation.

## Objects

<!-- @include: ../.fragments/docs/syntax-objects.md -->

Interfaces describe object shapes. Object literals are checked structurally against the declared fields.

## Modules

A module publishes names with `export`, either on the declaration itself or through a separate clause that may rename them. One name per module may be the default export.

<!-- @include: ../.fragments/docs/syntax/module-exports.md -->

`export protect` narrows who may import a name. A protected name is visible in the folder that declares it and in every subfolder of that folder, never in the parent folder and never in a sibling folder. The rule depends only on the declaring file, so moving a whole folder preserves every relation inside it.

<!-- @include: ../.fragments/docs/syntax/module-protect.md -->

Importing it from outside that folder is a compile error that names the symbol, the folder it is limited to, and the two ways out.

::: danger error
<!-- @include: ../.fragments/docs/syntax/module-protect-error.md -->
:::

A nested folder is a new, narrower boundary: a name protected in `src/auth/session/` is not visible in `src/auth/`, even through a barrel file in between. A re-export carries the original scope unchanged, so `export *` and `export { name } from` cannot widen it, and a namespace import simply omits the protected names. `protect` is rejected on a default export and on a re-export, since neither declares the name it publishes. Colocate tests that need a protected name inside the folder that owns it; there is no exemption for test files.

The import side has the matching forms. A default import binds the source module's default export under any local name, a namespace import binds every export at once, and both may be combined with named imports in one statement.

<!-- @include: ../.fragments/docs/syntax/module-imports.md -->

`tyrus.json` can declare static internal aliases in its `imports` object. Each key begins with `#` and maps a directory inside the project root. `#` maps the source root, while a longer key such as `#config` maps a nested directory. Longer alias prefixes take precedence over shorter ones. Aliases are expanded once at compile time and never resolve packages, URLs, or runtime values.

A namespace is resolved at compile time: `math.add(1, 2)` becomes a direct call to that module's function, and no namespace value exists at runtime. Naming a member the module does not export is a compile error that names both the module and the member.

A module can also forward another module's exports without binding them locally.

<!-- @include: ../.fragments/docs/syntax/module-reexports.md -->

::: warning `export *` never forwards a default
`export *` forwards named exports only; the source module's default export stays private. Two exports publishing the same public name are a compile error, not a silent last-one-wins.
:::

Native runtime APIs are imported through stable `tyrus:*` module names when the surface is implemented. Imports are resolved recursively at compile time; circular imports, duplicate names and imports of non-exported symbols are compilation errors.

## Safety Defaults

- Every statement and declaration ends with `;` (`E3218`), including a `const` bound to an arrow function and a destructuring declaration. A construct that closes itself needs no `;`: `if`, `else`, `while`, `do ... while`, `for`, `for ... of`, `for ... in`, `match`, `switch`, `loop`, `try`/`catch`/`finally`, a `defer` block, `interface`, a behavior `interface`, `function` and `enum`. The update clause of a classic `for`, written `i++`, is closed by the `)`.
- Variables must be initialized before use.
- Absence is represented explicitly by `None` and by optional types like `string?`; `null` is never a value.
- Non-Copy values follow ownership and move rules.

## Native Execution and Language Coverage

- Runtime APIs such as filesystem and HTTP are native features linked into the generated program.
- Optional values use familiar `T?`, `None`, `??`, and narrowing syntax; `null` is not a value and `Optional<T>` is the equivalent long form.
- Some syntax is parsed before it is fully lowered to native code; experimental pages call this out explicitly.
