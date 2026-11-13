---
title: Control Flow
description: Conditionals, loops, switch, match, return, throw, and try/catch/finally in tyrus.
---

# Control Flow

Tyrus supports familiar statement forms for branching, looping, and early exits. The checker validates initialization, ownership, and return paths.

## Conditionals

<!-- @include: ../.fragments/language/control-flow/conditionals.md -->

The condition is a `boolean`, or a `T?` used as a presence test: an optional is truthy when it holds a value. The same applies to `while`, `for`, the ternary `? :`, `&&`, `||`, and `!`. In the branch where the optional is known present, an `if` body, the truthy side of a ternary, or the right side of `&&`, it narrows to its non-optional type.

## Conditions and short-circuit statements

<!-- @include: ../.fragments/language/control-flow/conditions-and-short-circuit.md -->

`cond && effect();` and `cond || effect();` are also valid statements: they run the right side for its short-circuit side effect, so the right side may return `void`.

## while

<!-- @include: ../.fragments/language/control-flow/while-loop.md -->

## do...while

<!-- @include: ../.fragments/language/control-flow/do-while-loop.md -->

`do...while` runs the body at least once.

## loop

<!-- @include: ../.fragments/language/control-flow/loop.md -->

`loop` repeats its body forever until a `break`. Unlike `while`, it never evaluates a condition, so it is the natural form for loops that exit mid-body. When every `break` in the body carries a value, the `loop` is an expression: it evaluates to the value passed to the `break` that exits it, so `loop { break 42; }` has type `i32`. `continue` skips to the next repetition. A `break` with a value is only allowed inside a `loop` expression (`E0571`); `break` without a value always works as in any other loop.

## Classic for

<!-- @include: ../.fragments/language/control-flow/classic-for-loop.md -->

A counter written without an annotation takes its type from the loop bound: `i` above is the `usize` that `length` yields, so it indexes the array directly and needs no annotation of its own. A counter measured against a signed bound stays `i32`, which keeps arithmetic that dips below zero valid. Annotate the counter when you want to pick the type yourself; the annotation always wins.

## for...of

<!-- @include: ../.fragments/language/control-flow/for-of.md -->

Use `for...of` for arrays and iterable runtime values. The binding may destructure each element with an array/tuple pattern `[a, b]` or an object pattern `{ x, y }`, which pairs well with a `Map`'s `[key, value]` entries.

<!-- @include: ../.fragments/language/control-flow/for-of-destructuring.md -->

`for...of` also iterates a `Range` directly, binding each of its numeric values in turn, `a..b` excludes `b`, `a..=b` includes it.

<!-- @include: ../.fragments/language/control-flow/for-of-range.md -->

## for...in

<!-- @include: ../.fragments/language/control-flow/for-in.md -->

`for...in` iterates over the field names of an object. An array is not an object type, so reach for `keys()` with `for...of` when the index is the value you need; array indexes are `usize` in the native collection API.

<!-- @include: ../.fragments/language/control-flow/for-in-array-keys.md -->

## break and continue

<!-- @include: ../.fragments/language/control-flow/break-and-continue.md -->

`break` exits the nearest enclosing loop or `switch`. `continue` advances the nearest enclosing loop and also works from inside a nested `switch` or `match`. A `match` arm never falls through and rejects a `break` that could otherwise target an outer control structure (`E0269`). A `break` outside every loop and `switch`, or a `continue` outside every loop, is an error (`E0268`).

## return

<!-- @include: ../.fragments/language/control-flow/return.md -->

The checker rejects functions with non-`void` returns that can fall through without returning a value.

## `defer` and `errdefer`

<!-- @include: ../.fragments/language/control-flow/defer.md -->

`defer` registers one expression or a block to run when its current lexical scope exits. `errdefer` has the same two forms but runs only when that scope exits by an explicit error return or by propagating an error with `?`. Both are ambient language keywords and need no import. Finalizers run in LIFO order; on an error exit, the scope's `errdefer` finalizers run first, then its `defer` finalizers. Nested scopes complete before their parent scope.

The body captures values from its registration point. It cannot use `return`, `break`, `continue`, `throw`, `await`, or `?`, because cleanup must not replace the exit already in progress. `errdefer` requires a `Result<T, E>` or `T?` return type. Finalizers are currently supported in synchronous functions only. `panic()` aborts immediately and does not run either finalizer, so recoverable cleanup must use a structured error path.

## switch and match

`switch` is a control-flow statement for comparing one subject against literal cases. Use it for simple dispatch over primitive values or enums.

<!-- @include: ../.fragments/language/switch/basic-switch.md -->

::: tip Use switch for simple dispatch
Use `switch` when the subject is a primitive value and each branch is a literal equality check.
:::

### Fallthrough and Break

Execution falls through from a matched case into each following body. Use `break` to exit the nearest `switch` (or loop). Omitting it is useful when multiple cases share behavior:

<!-- @include: ../.fragments/language/switch/fallthrough.md -->

With `"admin"`, this prints both `privileged` and `authenticated`. `continue` inside a `switch` still advances the nearest enclosing loop. Unlike `switch`, [`match`](./pattern-matching.md) never falls through.

### Duplicate Cases

The linter reports repeated cases because the second branch is unreachable.

::: danger error
<!-- @include: ../.fragments/language/switch/duplicate-case.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/switch/duplicate-case-error.md -->
:::

### Switch vs Match

Use [`match`](./pattern-matching.md) when the branch needs destructuring, guarded arms, or narrowing over a tagged union. Use `switch` to dispatch over an enum or a literal value.

<!-- @include: ../.fragments/language/switch/enum-switch.md -->

::: warning Match patterns are not enum members
A `match` pattern is a wildcard, a binding, an object pattern, or a literal. An enum member such as `PaymentStatus.Paid` is not a pattern, so dispatch over an enum with `switch` and keep `match` for tagged unions and guarded arms.
:::

## throw and try/catch/finally

<!-- @include: ../.fragments/language/control-flow/try-catch-finally.md -->

A `catch` without a type binds `Error`, the interface with `message` and `code` that recoverable errors use, so `error.message` and `error.code` are available without an annotation. A thrown `string` or any other type still needs its type written, as in `catch (error: string)`.

<!-- @include: ../.fragments/language/control-flow/catch-error.md -->

The `finally` block runs before control leaves `try` or `catch`, including through `return`, `throw`, `break`, or `continue`.

::: warning Avoid unsafe finally flow
Returning, throwing, breaking, or continuing from `finally` can override the original control flow and is reported by lint rules.
:::
