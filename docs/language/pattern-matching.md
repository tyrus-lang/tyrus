---
title: Pattern Matching
description: match expressions, guarded arms, wildcard arms, union narrowing, and exhaustiveness.
---

# Pattern Matching

`match` is a native tyrus control-flow form that checks whether every possible case is handled.

Arms use `pattern => body` directly. For a single returned expression, use the compact form `pattern => expression;`; its semicolon is required. Block arms remain supported, and their trailing semicolon is optional. Parentheses around a pattern are optional for every supported pattern, so both `"ready" =>` and `("ready") =>` are equivalent.

<!-- @include: ../.fragments/language/pattern-matching/pattern-matching.md -->

::: warning A compact arm is a `return`
`pattern => expression;` returns `expression` from the enclosing function, exactly like `return expression;`. Inside a loop it therefore leaves the whole function, not just the loop, and nothing after the `match` runs. The program below prints only `one`. To run the arm and continue, use a block arm, `1 => { print("one"); }`.

<!-- @include: ../.fragments/language/pattern-matching/compact-arm-returns.md -->
:::

## Match over Unions

<!-- @include: ../.fragments/language/pattern-matching/match-over-unions.md -->

Supported patterns include literal matches, wildcard `_`, capture bindings, guarded arms, object destructuring with literal field matching, and array patterns.

## Array Patterns

Array patterns destructure `Array<T>` and `readonly Array<T>` values. `[]` matches only an empty array, `[first, second]` matches exactly two elements, and `[head, ...tail]` matches one or more elements. Element bindings have type `T`; `tail` is a new owned `Array<T>` containing the remaining elements, equivalent to `values.slice(1)`, rather than a borrowed view. An array pattern permits at most one rest pattern, it must be final, and it must be either a binding or `_`. Literal elements must be assignable to `T`. Nested array patterns and length-based exhaustiveness remain unavailable because ordinary arrays do not carry their length in the type.

<!-- @include: ../.fragments/language/pattern-matching/array-patterns.md -->

## Literal Matching

Match directly against constant values like strings, numbers, or booleans.

<!-- @include: ../.fragments/language/pattern-matching/literal-match.md -->

## Range Patterns

Match a numeric or `char` value against a bounded interval: `a..b` excludes `b`, `a..=b` includes it, and either bound may be omitted for an open range (`..b`, `a..`). A range bound must be assignable to the subject's own type, matching a `string` against a numeric range is a compile error.

<!-- @include: ../.fragments/language/pattern-matching/range-pattern.md -->

An empty or inverted range (`10..5`) is a warning, not an error, it simply never matches, the same as any other unreachable arm. Detecting overlapping arms and verifying exhaustiveness for a finite numeric type (`u8`, for example) are deliberate scope decisions, not gaps: both remain open, tracked refinements rather than required behavior.

## Binding Patterns

Capture matched values into variables for use in the arm body.

<!-- @include: ../.fragments/language/pattern-matching/binding-pattern.md -->

## Wildcard Pattern

Use `_` to match any value that wasn't matched by previous arms.

<!-- @include: ../.fragments/language/pattern-matching/wildcard-pattern.md -->

## Guarded Arms

Add `if` conditions to arms for more specific matching.

<!-- @include: ../.fragments/language/pattern-matching/guard-pattern.md -->

## Object Destructuring

Destructure objects and match against specific field values.

<!-- @include: ../.fragments/language/pattern-matching/object-destructure.md -->

## Combined Patterns

Combine type narrowing, guards, and literal matching for complex logic.

<!-- @include: ../.fragments/language/pattern-matching/combined-patterns.md -->

## Enum Variant Patterns

Match an algebraic enum by variant and bind one name per payload field. A variant with two fields needs two bindings: a single name for a multi-field payload is rejected, because there is no way to read the fields back out of it. A variant without a payload takes no binding at all.

<!-- @include: ../.fragments/language/pattern-matching/enum-variant-pattern.md -->

## Exhaustiveness

For closed subjects such as enum, boolean, literal union and tagged union values, `match` should cover every variant or include `_`. Unreachable arms are reported as warnings.

::: tip Switch is covered in Control Flow
Use [`switch`](./control-flow.md#switch-and-match) for literal dispatch over primitive values. Use `match` when destructuring, narrowing or exhaustiveness matters.
:::
