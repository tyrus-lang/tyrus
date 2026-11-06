---
title: Operators and Expressions
description: Numeric, logical, bitwise, ternary, pipeline, typeof, type assertions, spread, conversion, parsing, and process exit.
---

# Operators and Expressions

Tyrus keeps familiar operators where the semantics can be checked statically and lowered to native code.

## Operator Groups

| Category | Operators |
| --- | --- |
| Arithmetic | `+` `-` `*` `/` `%` `**` |
| Unary | `-` `+` `++` `--` `!` `not` `~` |
| Bitwise | `&` `\|` `^` `~` `<<` `>>` `>>>` |
| Comparison | `>` `<` `>=` `<=` `===` `!==` |
| Logical | `&&` `and` `\|\|` `or` |
| Ternary | `? :` |
| Assignment | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `>>>=` `&&=` `\|\|=` `??=` |
| Null safety | `??` `?.` |
| Borrow | `&` `&mut` |
| Membership | `in` `not in` |
| Type check | `is` `not is` |
| Range | `..` `..=` |

`and`, `or`, and `not` are exact written equivalents of `&&`, `||`, and `!`, the lexer maps each pair to the same token, so both spellings can be mixed freely in the same expression: `active and not suspended or isAdmin` and `active && !suspended || isAdmin` compile identically.

## Chained Comparisons

Three or more of `<`, `<=`, `>`, `>=`, `===` chained directly (`a < b < c`) are sugar for `a < b && b < c`, reading closer to algebraic notation than the equivalent written with `&&`.

<!-- @include: ../.fragments/language/operators/chained-comparison.md -->

Each term in the chain is evaluated exactly once, even when it takes part in two comparisons, a term with a side effect (a function call) never runs twice. The chain also short-circuits like `&&`: once one comparison fails, later terms are never evaluated. Two terms (`a < b`) are unaffected by this sugar and stay a plain comparison; the chain only applies at three terms or more. `!==` never chains, there is no algebraic reading of `a != b != c`, so it always stays a plain, two-term comparison.

## Pipeline

See the [Pipe Operator guide](./pipe-operator.md) for the `|>` operator, the `%` topic token, and pipeline composition.

## Ternary

<!-- @include: ../.fragments/language/operators/ternary.md -->

The ternary condition must be `boolean` and both branches must have the same type.

## typeof

The `typeof` operator returns a `string` naming the tyrus type of a value at runtime. Numeric types stay distinct instead of collapsing to a general number category, an array reports its element type, an interface reports its name, and an optional keeps its `?`. Only two shapes answer with a category, because they have no name to give: an anonymous object type (`"object"`) and a function (`"function").

<!-- @include: ../.fragments/language/operators/typeof-values.md -->

`typeof` can also be used as a type query.

<!-- @include: ../.fragments/language/operators/typeof-type-query.md -->

`typeof` comparisons narrow union values in control flow.

<!-- @include: ../.fragments/language/operators/typeof-narrowing.md -->

This works with enums as well, `typeof` returns the underlying representation:

<!-- @include: ../.fragments/language/operators/typeof-enum.md -->

## is / not is

The `is` operator checks whether a runtime value matches a specific type within a union, returning `boolean`. It is the primary mechanism for type narrowing on union types.

<!-- @include: ../.fragments/language/operators/is-narrowing.md -->

Inside the `true` branch, the compiler narrows `value` to `string`. Outside, it remains `string | number`.

`not is` is the negated form:

<!-- @include: ../.fragments/language/operators/is-not-is.md -->

`is`/`not is` also work in `while` conditions and boolean assignments. The check runs at runtime by comparing the union's type tag against the target type.

A caught error keeps the type it was thrown as, so `is` also tells errors apart. In `catch (error)`, `error is ConfigError` is `true` when the value thrown was a `ConfigError` (a module error such as `FsError`, or an interface that extends `Error`), and inside that branch `error` has every field of the subtype. A `catch (error: A | B)` narrows to the variant the same way. This holds in an `async` function too, including for an error that arrives as a rejected `await`, and in a `catch (error)` that awaits before checking. Only an `Error` that did not come from a `catch` cannot tell its subtype (`E3297`). A handler that checks for the errors it knows ends with `throw error;` for the rest: the value goes on with its subtype, so the next `catch` still tells it apart, and an error type added later is passed on instead of silenced.

<!-- @include: ../.fragments/language/operators/is-caught-error.md -->

## Type Assertions

<!-- @include: ../.fragments/language/operators/type-assertions.md -->

A numeric assertion preserves the value or aborts; it never wraps. The checks run in every profile, so a debug run predicts what the shipped binary does. Non-primitive assertions are identity casts and should be used carefully.

## Conversion and Parsing

<!-- @include: ../.fragments/language/operators/conversion-parsing.md -->

Parsing returns optional values instead of `NaN`, preserving null safety.

The annotation picks the numeric width used by the parse. `parseInt` narrows to any integer type and `parseFloat`/`Number` to `f32` or `f64`; with no annotation they stay `i64?` and `f64?`. Text whose value does not fit the target simply does not parse as that type, so the result is absence rather than a truncated value.

<!-- @include: ../.fragments/language/operators/parse-width.md -->

A plain `as T` preserves the value or aborts; it never wraps around. What fits depends on the bounds of both types rather than on their widths, so a `u32` holds a value above `i32::MAX` while a negative number never fits a `u32`, not even one of the same width.

`as T?` is a fallible cast and follows the same rule, except that a value outside the range of `T` becomes absence instead of aborting, which is how a program recovers from one. A cast from a float to an integer truncates toward zero, and `NaN` yields absence.

<!-- @include: ../.fragments/language/operators/fallible-cast.md -->

`as` converts between numbers; it never reinterprets a value as an unrelated type. An enum reads back as the primitive it is represented by, but nothing else crosses: a string is not a number, and one interface is not another even when their fields line up. A value of the target type has to be *built*, and asserting it instead is an error.

::: danger error
<!-- @include: ../.fragments/language/operators/interface-assertion-error.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/operators/validation-error-output.md -->
:::

This matters most where a type carries validators. A `User` is guaranteed to satisfy `@min(18)`, and that guarantee is what lets the rest of the program skip the check. A conversion that produced a `User` without running its validators would make the guarantee a lie, so it is rejected: build the `User`, and handle the `ValidationError` it may throw.

## Spread and Rest

The spread operator expands arrays and objects into individual elements or fields:

<!-- @include: ../.fragments/language/operators/spread.md -->

Rest parameters collect trailing arguments into an array:

<!-- @include: ../.fragments/language/operators/rest.md -->

## Membership and Ranges

`in` and `not in` check whether a value belongs to an array, a `Range`, a `Map`, or a `Set`, replacing `.includes()`/`!.includes()` and `.has()`/`!.has()` calls. `not in` is the negated form, spelled with the `not` keyword rather than a separate operator.

`a..b` is a `Range` from `a` up to, but excluding, `b`; `a..=b` includes `b`. A range can be annotated as `Range<T>`, such as `const ids: Range<i32> = 1..4;`. A `Range` has its own `.contains(x)` method, `x in a..b` and `(a..b).contains(x)` are the same check.

<!-- @include: ../.fragments/language/operators/membership-and-ranges.md -->

`in`/`not in` also keep their original meaning over a plain object or interface value: `"field" in obj` checks whether `obj`'s type declares a field named `"field"`, independent of the array/Range/Map/Set membership check above.
