---
title: Types
description: Primitive, numeric, optional, object, array, tuple, union, and function types in tyrus.
---

# Types

Tyrus uses familiar type annotations with native semantics enforced by the compiler. Values can often be inferred from initializers, but explicit annotations are encouraged at API boundaries and when numeric width matters.

::: tip Inference is local and predictable
`const answer = 42;` is inferred from the literal. Use `const answer: i64 = 42;` when the width is part of the contract.
:::

::: warning An object literal names its interface
Interfaces are nominal, so a literal states the interface it builds instead of having one guessed from its shape: `const user: User = { id: 1 };`. An unannotated object literal is an error whose `help:` names every interface with a matching shape, offered in the editor as a Quick Fix. Guessing by shape would make a file's meaning depend on every interface in the program.
:::

## Primitive Types

| Type | Use it for | Example |
|------|------------|---------|
| `boolean` | Truth values and branch conditions. | `const enabled: boolean = true;` |
| `i8` | Bytes and small signed values, one byte wide. | `const delta: i8 = -8;` |
| `i16` | Small signed values, two bytes wide. | `const offset: i16 = -1024;` |
| `i32` | Default signed integer for counters, IDs, and most arithmetic. | `const count: i32 = 42;` |
| `i64` | Large signed integers, timestamps, and persisted identifiers. | `const micros: i64 = 1720000000000;` |
| `i128` | Very large signed integer calculations. | `const ledger: i128 = 9000000000000000000;` |
| `isize` | Pointer-sized signed indexes and native-size values. | `const offset: isize = -1;` |
| `u8` | Bytes, protocol codes, and color channels. | `const red: u8 = 255;` |
| `u16` | Ports, code points, and two-byte protocol fields. | `const port: u16 = 8080;` |
| `u32` | Unsigned counters and sizes that fit in 32 bits. | `const size: u32 = 4096;` |
| `u64` | Large unsigned counters and identifiers. | `const sequence: u64 = 10000000000;` |
| `u128` | Very large unsigned integer calculations. | `const mask: u128 = 340282366920938463463374607431768211455;` |
| `usize` | Pointer-sized unsigned indexes, lengths, and capacities. | `const length: usize = names.length;` |
| `f32` | Compact floating-point values where precision tradeoffs are acceptable. | `const ratio: f32 = 0.75;` |
| `f64` | Default floating-point calculations. | `const total: f64 = 19.99;` |
| `f128` | Extended floating-point calculations and native min/max constants. | `const largest: f128 = f128.MAX;` |
| `decimal` | Exact base-ten arithmetic with 34 significant digits. | `const total: decimal = 0.1d + 0.2d;` |
| `char` | A single Unicode character, for iterating text one character at a time. | `const letter: char = 'A';` |
| `string` | Owned UTF-8 text. | `const label: string = "tyrus";` |
| `void` | Functions that do not return a value. | `function log(): void { print("ok"); }` |
| `unknown` | Values that must be narrowed before use. | `const payload: unknown = readInput();` |
| `never` | Code paths that do not return normally. | `function fail(message: string): never { throw message; }` |

## Numeric Types

Use signed integers (`i*`) when values can be negative. Use unsigned integers (`u*`) for bytes, sizes, protocol fields, and values that must never be negative. Use `f32`/`f64` for fractional values, not money or exact counters.

<!-- @include: ../.fragments/language/types/numeric-types.md -->

## Algebraic floating-point methods

`f32` and `f64` provide `algebraicAdd`, `algebraicSub`, `algebraicMul`, `algebraicDiv`, and `algebraicRem`. They correspond respectively to addition, subtraction, multiplication, division, and remainder, while explicitly allowing the compiler to reassociate the selected floating-point work for optimization. Results can vary between builds and targets, including for `NaN`, infinities, and signed zero. The ordinary arithmetic operators remain strict.

<!-- @include: ../.fragments/language/types/algebraic-float-methods.md -->

## Special floating-point values

`Infinity` and `NaN` are reserved floating-point literals available without an import. `-Infinity` is unary `-` applied to `Infinity`. They infer `f64` and may be contextually typed as `f32`, `f64`, or `f128`. Floating-point arithmetic follows IEEE 754, including non-finite results; integer division by zero remains a runtime error.

<!-- @include: ../.fragments/language/types/special-floating-point-values.md -->

`isNaN(value)` accepts one floating-point value and returns `boolean`. Equality cannot identify `NaN`: every equality comparison with it is false, while every inequality comparison with it is true.

::: warning Numeric width is part of the type
Passing an `i64` where an `i32` is expected requires an explicit narrowing conversion. The compiler rejects implicit narrowing because it can lose data.
:::

::: danger error
<!-- @include: ../.fragments/language/types/narrowing-example.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/types/narrowing-error.md -->
:::

## Numeric Constants

Numeric types expose `MIN` and `MAX` constants through type-name property syntax.

<!-- @include: ../.fragments/language/types/numeric-constants.md -->

## Numeric literal syntax

Tyrus supports hexadecimal, binary, octal, and decimal integer literals, with optional underscores as visual separators. Hexadecimal prefixes and digits are case-insensitive. Decimal integer separators must group digits in sets of three. Positive integer literals above `i128.MAX` require an explicit `u128` context and cannot exceed `u128.MAX`; the signed negative boundary is `i128.MIN`. Scientific notation is available for decimal floating-point literals, and large integer parts are not implicitly widened to `f128`.

<!-- @include: ../.fragments/language/types/numeric-literals.md -->

## Decimal parsing

`parseDecimal(text)` parses a complete base-ten decimal string and returns `decimal?`. Invalid text, a `d` or `D` suffix, values outside the decimal range, and malformed separators produce absence instead of a binary floating-point approximation. Integer-to-decimal casts are exact; a decimal cast to an integer requires an integral value and a destination range that can hold it.

<!-- @include: ../.fragments/language/types/decimal-parsing.md -->

Unsigned integer `MIN` is always zero. `isize` and `usize` are as wide as a pointer on the machine the program is built for, so their bounds are not fixed by the language: they expose no `MIN` or `MAX`, and a fixed-width type should be used when the limit itself matters.


::: danger An integer literal must fit its type
`const byte: u8 = 300;` is a compile error (`E0706`) anchored on the literal, never a value that wraps silently. The editor offers a quick fix that widens the type to the narrowest one holding the value.
:::
Numeric literals may carry an explicit integer or floating-point type suffix. The suffix fixes the literal type, participates in inference, and is distinct from an explicit `as` conversion.

<!-- @include: ../.fragments/language/types/typed-numeric-literals.md -->
## Characters

A `char` holds exactly one Unicode character and is never inferred: it arrives through an annotation, and the value is written as a one-character literal. Text of any other length is a `string`.

<!-- @include: ../.fragments/language/types/char.md -->

The quotes carry no meaning, the annotation is what makes a value a `char`, but `tyrus fmt` writes a `char` with single quotes and a `string` with the configured quote style, so the two read differently at a glance.

"One Unicode character" means one Unicode scalar value, a single code point, not a grapheme cluster, which is what a reader perceives as one symbol. Some emoji are a single scalar value and fit in a `char`; many are written with several code points and must be a `string`:

| Emoji | Code points | Count | `char` |
|-------|-------------|-------|--------|
| 😀 | `U+1F600` | 1 | ✅ |
| 🇧🇷 | two regional indicators (`U+1F1E7` `U+1F1F7`) | 2 | ❌ |
| ❤️ | heart plus variation selector (`U+2764` `U+FE0F`) | 2 | ❌ |
| 👍🏽 | thumbs up plus skin-tone modifier | 2 | ❌ |
| 👨‍👩‍👧‍👦 | family, a ZWJ sequence | 7 | ❌ |

So `const emoji: char = '😀';` is accepted, while a composed emoji such as `'🇧🇷'` fails at compile time, `a char holds exactly one character, but 🇧🇷 has 2`, and the diagnostic points you to `string`.

`char` carries `toUpperCase`, `toLowerCase`, `isDigit`, `isAlphabetic`, `isAlphanumeric`, `isWhitespace` and `toString`. Case and classification follow Unicode's rules, not ASCII's, so `'ç'.toUpperCase()` is `'Ç'`. Converting an integer into a `char` is checked: a value that is not a Unicode character aborts, and `as char?` yields absence instead.

## Unknown values

`unknown` holds a value whose type is only known at run time. It accepts a `string`, a `boolean`, a `char`, any numeric type or a `decimal`, including a union of them, and nothing can be done with the value until a `typeof` check proves what it holds. In the branch where `typeof value === "string"` is true, `value` is a `string`. The comparison can be written in either order, and the `else` branch of `typeof value !== "string"` is narrowed the same way.

<!-- @include: ../.fragments/language/types/unknown-narrowing.md -->

Outside the proven branch the value stays `unknown`. A check for `"object"` or `"function"`, or the branch where a check failed, proves no single type and leaves the value `unknown`. Comparing `typeof` with a name it never produces, such as `"number"` or `"String"`, is a compile error (`E0795`) anchored on the literal, and the editor offers a quick fix when exactly one category is meant.

Arrays, objects, interfaces, optionals and functions cannot be stored in `unknown` (`E0794`). Give the value a type that describes it instead.

::: danger error
<!-- @include: ../.fragments/language/types/unknown-composite-error.md -->
:::

## Optional Types

`T?` represents a value that may be absent. tyrus has no `undefined` and no `null` value: absence is not a value, it is the state of a `T?` that has no value.

<!-- @include: ../.fragments/language/types/optional-basic.md -->

Because absence is not a value, you produce it in one of four ways: omit an optional field or optional parameter, write a bare `return;` in a function whose return type is `T?`, declare an optional binding without an initializer (`let x: T?;` starts empty), or call a native method that finds nothing (`array.find`). There is no `null` literal to write.

<!-- @include: ../.fragments/language/types/optional-absence.md -->

Read members of an optional value with `?.`. The access short-circuits when the receiver is absent, and its result stays optional: `name?.length` is `usize?`, not `usize`.

<!-- @include: ../.fragments/language/types/optional-chaining.md -->

::: danger Handle optional values before use
Forced unwrap with `value!` is rejected by the compiler (`E0716`) because it can turn absence into a runtime failure. Narrow the value with `if (value)` or supply a fallback with `??`.
:::

Optional types compose with collections and function returns:

<!-- @include: ../.fragments/language/types/optional-composition.md -->

## Result values

`Result<T, E>` represents either a success payload of exactly `T` or an error payload of exactly `E`. Every value returned from a `Result` function must be wrapped in `Ok(value)` or `Err(value)`, so the selected variant is explicit even when `T` and `E` differ. The `?` postfix extracts the success payload and returns the error from the current function when needed.

<!-- @include: ../.fragments/language/types/result-values.md -->

The payload types must be exact, and optional payloads such as `Result<User?, string>` are rejected. `Result<void, E>` returns success with `Ok()`. A bare return is rejected with a Quick Fix to use `Ok()`; a bare `return value;` is rejected with a Quick Fix that wraps the value in the only matching constructor, or offers both constructors when both payload types match. A result is narrowed with explicit `Ok(binding)` and `Err(binding)` match patterns, and both variants are required unless an explicit `_` arm covers the remaining case. `Ok()` is available when the success payload is `void`. The `?` operator is the explicit alternative when the error should be propagated to the caller.

`?` is a postfix operator, so the value it unwraps can take part in any expression: `load()? + 1`, `load()? === 40`, or a ternary arm such as `wanted ? load()? : 0`. It shares its token with the ternary, and the two are told apart by what follows: a `?` whose next expression is followed by `:` opens a ternary, so `flag ? -1 : 2` stays a ternary while `load()? - 1` propagates.

<!-- @include: ../.fragments/language/types/try-in-expressions.md -->

In an `async` function, `await` covers its operand up to the first `?`, so `await loadUser(id)?` waits for the promise and then propagates the `Result`: `const user = await loadUser(id)?;` or `return Ok(await load()? + 1);`. Parentheses keep their meaning, so `await (load()?)` applies `?` to the promise itself, which is rejected with a help that points to `await load()?`. An `await` may also be the subject of a `match` or an `if`, or an argument of a call, including inside a loop.

<!-- @include: ../.fragments/language/types/result-in-async.md -->

## Inline failure handling

`onfail` is a terminal guard attached to a simple `const` or `let` initializer. It handles `Result<T, E>`, an awaited `Promise<Result<T, E>>`, a direct call with one declared `throws E` effect, or an awaited rejection. The declared name is the error inside the handler and the success value after it. A handler never continues with a replacement value: it leaves with `return`, `throw`, `continue` or `break`, and reaching its end leaves for it, returning from a `void` function, returning `Err(name)` from a function that returns `Result<U, E>` with the same error type, or propagating the allowed exception. In a function that returns anything else, the handler must leave explicitly (`E0277`).

<!-- @include: ../.fragments/language/types/inline-failure-handling.md -->

Over an awaited `Result`, the `Err` is separated after the `await`, so the same guard handles native module failures. The handler itself runs without suspending, and an `await` inside it is refused (`E3002`).

<!-- @include: ../.fragments/language/types/inline-failure-handling-async.md -->

Use `try`/`catch` for multiple protected operations, recovery with a replacement value, or a `finally` block. An enclosing `finally` still runs exactly once when an `onfail` handler exits.

## Object Types

<!-- @include: ../.fragments/language/types/object-types.md -->

Object literals are checked structurally against interface declarations. Optional fields are accessed as optional values.

## Arrays and Tuples

<!-- @include: ../.fragments/language/types/arrays-tuples.md -->

Arrays hold a repeated element type. Tuples hold a fixed shape with a known type per position.

Use `[value; count]` to create an array where every element is the same value. The count must be a constant integer known at compile time. Both Copy types (integers, booleans) and non-Copy types (strings, nested arrays) are supported; non-Copy elements are deep-cloned for each position.

Use `Array.withCapacity(n)` to create an empty array with pre-allocated buffer space. The element type is inferred from the variable annotation. This avoids repeated reallocations when you know the approximate number of elements you will insert.

## Generic Type Constructors

Use angle brackets to supply type arguments to named type constructors. `Array<T>` is equivalent to `T[]`, and `Tuple<T, ...>` is equivalent to `[T, ...]`; choose the spelling that makes the surrounding type easier to read.

| Form | Arguments | Meaning |
| --- | --- | --- |
| `Array<T>` | one | A mutable or immutable sequence of `T`; equivalent to `T[]`. |
| <code style="white-space: nowrap">Tuple&lt;T, ...&gt;</code> | <span style="white-space: nowrap">zero or more</span> | A fixed-length sequence with one type per position; equivalent to `[T, ...]`. |
| `Optional<T>` | one | A value that may be absent; equivalent to `T?`. |
| `Range<T>` | one | A numeric range produced by `a..b` or `a..=b`. |
| `Set<T>` | one | A collection of unique values of `T`. |
| `Map<K, V>` | two | A keyed collection from `K` to `V`. |
| `Record<K, V>` | two | An object with keys `K` and values `V`; `Record<string, V>` is a dynamic map. |
| `Promise<T>` | one | The result of asynchronous work that resolves to `T`. |

The object type utilities are also generic: `Partial<T>`, `Required<T>`, `Readonly<T>`, `DeepPartial<T>`, `Pick<T, K>`, `Omit<T, K>`, `PickByType<T, Type>`, `OmitByType<T, Type>`, `CamelCaseKeys<T>`, `SnakeCaseKeys<T>`, `Require<T, K>`, `AtLeastOne<T, K>`, `DataOnly<T>`, `Exclude<T, U>`, `Extract<T, U>`, `ReturnType<T>`, and `Parameters<T>`. See [Collections](./collections), [Objects and Interfaces](./objects), and [Async and Await](./async-await) for their operations and constraints.

## Function Types

<!-- @include: ../.fragments/language/types/function-types.md -->

Function signatures include parameter and return types. The compiler uses those signatures for call checking, completion, hover, and signature help.

Function types compose with other type constructs:

<!-- @include: ../.fragments/language/types/function-composition.md -->

Functions can be stored in arrays and maps, enabling callback registries and event systems:

<!-- @include: ../.fragments/language/types/function-registries.md -->

Parentheses group a type, so an array of function type can be written inline instead of only through an alias:

<!-- @include: ../.fragments/language/types/parenthesized-types.md -->

What tells a group from a parameter list is the arrow, and it is only readable after the closing parenthesis: `(n: i32) => i32` is a function type, `(i32)[]` is an array of `i32`. A group holds one type, unnamed and without `...`; anything else is a parameter list and still needs its arrow.


A function type can declare what the function may throw: `(string) => i32 throws string`. A call through a value of that type has to catch or propagate those errors, like a direct call, and a function that throws cannot be used where the function type does not declare it (`E3298`). An `async` function reports its errors as a rejection instead, so it fits a function type without `throws`.

<!-- @include: ../.fragments/language/types/function-throws.md -->
