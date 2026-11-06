---
title: Objects and Interfaces
description: Interfaces, type aliases, optional and readonly fields, object updates, unions, and type utilities.
---

# Objects and Interfaces

Tyrus uses concise object syntax, while named interfaces and object aliases lower to native struct layouts.

## Interfaces

<!-- @include: ../.fragments/language/objects/interfaces.md -->

`readonly` prevents field reassignment. Optional fields read as `T?` and must be handled with narrowing or `??`. Object literals are checked for missing, extra and incorrectly typed fields. Object spread creates immutable updates with the same interface layout as the source object. When a field's value is a local variable with the same name, write the name once, `{ active }` is shorthand for `{ active: active }`, and mixes freely with explicit `key: value` fields and with a spread update.

## Automatic Data Methods

Every named data interface receives automatic data methods without annotations or declarations. The first four are regular instance calls, and `fromJson` is a static call on the concrete interface name. A behavior interface can define an instance method with the same name and takes precedence over the automatic method.

### `copy()`

Creates an independent structural copy.

`copy()` is not only for objects: it exists on every value that owns a buffer, which is every type that is not `Copy`. That covers `string`, arrays, tuples, and the optionals, results and unions built from them. On a `Copy` scalar such as `i32` there is no method, because `let a = b` already duplicates one and a method would suggest something else happens.

The result is a plain value of its own type. A decoration belongs to the field a value was read from, not to the value, so nothing about the source's declaration follows the copy.

Reading a field into a binding of the same type moves it: `const items = order.items;` hands the field to `items`, `order` stays usable through its other fields, and reading `order.items` or `order` as a whole afterwards is refused (`E0382`) until the field is assigned again. Where a move cannot happen, the compiler suggests `copy()` instead: when the owner's fields carry validators, whose rules describe the value as a whole, when the field is reached through a reference, or when the binding is declared with a different type than the field, such as a `string` read out of a narrowed `string?` (`E3081`). `order.items.copy()` says what it does and keeps the cost visible, which a copy inserted silently would not.

<!-- @include: ../.fragments/language/objects/automatic-data-copy.md -->

### `equals(other: Type)`

Compares eligible fields structurally, including absent and present optional fields. Arrays are not eligible because their equality is by address. The argument must have the same nominal interface type.

<!-- @include: ../.fragments/language/objects/automatic-data-equals.md -->

An interface with matching fields still cannot be passed to `equals()` when it has a different name:

::: danger error
<!-- @include: ../.fragments/language/objects/automatic-data-methods-equals-error.md -->
:::

### `toString()`

Produces a readable representation in declared field order, quotes and escapes strings, renders absent optional fields as `None`, and recursively renders nested values.

<!-- @include: ../.fragments/language/objects/automatic-data-to-string.md -->

### `toJson(options?: { pretty: boolean })`

Encodes canonical JSON with a space after each colon. Pass `{ pretty: true }` for multi-line JSON with four-space indentation. Keys are written in sorted order, not declaration order, so the same data always produces the same text: adding a field later does not move the ones already there. A `decimal` is written in plain notation, the same text it prints as: `12.50d` is `12.5`, with every digit preserved and no rounding.

<!-- @include: ../.fragments/language/objects/automatic-data-to-json.md -->

A field holding a discriminated union is written as the object its live variant already is: there is no envelope, and the discriminant field is what tells a reader which variant it got. `fromJson` reads one back the same way, choosing the variant by its discriminant; a discriminant naming no declared variant is refused with `ERR_JSON_UNKNOWN_VARIANT` rather than guessed at, which is the whole reason for writing it.

<!-- @include: ../.fragments/language/objects/automatic-data-to-json-union.md -->

### `Type.fromJson(text: string)`

Validates syntax, fields, types and numeric ranges, returning `JsonError` with stable `code`, readable `message`, and field `path` instead of throwing.

<!-- @include: ../.fragments/language/objects/json-error.md -->

<!-- @include: ../.fragments/language/objects/automatic-data-from-json.md -->

Use `onfail` to print a `JsonError` and stop the current `void` function when decoding fails:

::: danger error
<!-- @include: ../.fragments/language/objects/automatic-data-from-json-onfail.md -->
:::


## Behavior Interfaces with `at`

Behavior interfaces attach executable methods to one or more receiver types with `at`. The receiver is an explicit first parameter, so the method remains ordinary and its call-site syntax is `value.method()`. When its annotation is omitted for multiple targets, the compiler specializes and checks the implementation separately for every target. A behavior interface contains methods or shorthand bindings, not data fields.

<!-- @include: ../.fragments/language/objects/behavior-interfaces.md -->

Shorthand bindings name functions declared in the same module or imported from another module. Their signatures are checked when the behavior is declared, and the imported signature is retained across module boundaries for type checking and native code generation. Behavior interfaces can also `extend` other behavior interfaces to inherit their methods.

The receiver of `at` may be a named interface or a primitive type. The same member-call syntax works for numeric values, strings, booleans and chars.

## Property access by name

A field is read and written with the dot form, and with a string literal in brackets, the two are the same access: the key is written in the source, so it names a field the compiler knows, and both compile to the same fixed offset.

<!-- @include: ../.fragments/language/objects/literal-property-access.md -->

Only a literal qualifies. `person[key]`, where `key` is a variable, is a compile error (`E0608`): an interface has a fixed set of fields and no run-time lookup to fall back on. A literal that names no field is `E0609`, reported with the name. For keys decided at run time, use `Record<string, T>`, which is a real hash map, `lookup["a"]` there is a lookup that can miss, so it yields `T?`.

## Field assignment

A field can be assigned in place with `object.field = value`, including compound forms such as `+=`, through nested receivers, and on indexed elements.

<!-- @include: ../.fragments/language/objects/field-assignment.md -->

In-place assignment requires a mutable binding: a `const` binding is deeply immutable, and a `readonly` field or a value of a `readonly` type rejects the write. In-place assignment currently works for number, boolean and char fields; to change a `string`, an array or a nested object field, build a new value with object spread.

## Interface Inheritance

An interface can `extend` one or more other interfaces to inherit their fields. The child interface requires every inherited field plus its own, so a value of the child type must provide all of them. Inheritance only merges fields, it does not make the child assignable to the parent (see the nominal layout below).

<!-- @include: ../.fragments/language/objects/interface-inheritance.md -->

## Type Intersection

A `type` alias combines object types with `&`. The alias requires every field of every operand, exactly as inheritance does, and each operand is either a named interface or an object written inline. `&` is the alias form of what `extend` does for an interface: `interface` inherits with `extend`, `type` combines with `&`.

<!-- @include: ../.fragments/language/objects/type-intersection.md -->

When two operands declare the same field with different types, the alias is rejected where it is written (`E3214`) instead of producing a field no value can ever satisfy.

::: danger error
<!-- @include: ../.fragments/language/objects/type-intersection-conflict.md -->
:::

`&` belongs to a `type` alias and nowhere else. It is not accepted in a type annotation (`E3215`): name the intersection first, then use the name.

::: danger error
<!-- @include: ../.fragments/language/objects/type-intersection-annotation.md -->
:::

Every operand must be an object type. An operand that is a primitive, an array, a union or a function is rejected with `E3213`, and an alias that refers to itself through the chain is rejected with `E3216`. Mixing `&` and `|` in the same alias is rejected with `E3217`, which is its own diagnostic because the reason is the precedence: `|` binds tighter, so `type A = B & C | D;` would read as `B & (C | D)`. Parentheses alone do not help, since a union is never a valid operand. Name the intersection and use that name in the union: `type BC = B & C;` and then `type A = BC | D;`.


## Nominal Native Layout

Matching object shapes are not interchangeable. Interfaces are native named product types, so passing a `CompleteUser` where `SummaryUser` is expected requires an explicit conversion object.

<!-- @include: ../.fragments/language/objects/nominal-native-layout.md -->

To hold values of several shapes in one collection, use a tagged union with a literal discriminant field and `match` instead of a shared base interface, it keeps every variant's fields and stays exhaustive:

<!-- @include: ../.fragments/language/objects/tagged-union-narrowing.md -->

### Field Storage Order

Fields are stored in whatever order wastes the least space, which is not always the order they are written: a wide field is placed ahead of a narrow one so the gap the narrow one would leave in front of it disappears. The interface below is 40 bytes written this way and 24 bytes as stored, and holding a million of them is the difference between 40 MB and 24 MB.

<!-- @include: ../.fragments/language/objects/field-storage-order.md -->

Nothing about using the interface changes: fields are read, written, spread and destructured by name, an object literal still evaluates its fields in the order it lists them, and printing a value still shows the fields in the order the interface declares them. The order is a storage decision and is never observable from the program.

## Type Aliases

<!-- @include: ../.fragments/language/objects/type-aliases.md -->

For object shapes, `type` and `interface` are equivalent in native layout. A `type` alias can also name unions, tuples, function types and utility types.

## Type Utilities

| Category | Utilities |
| --- | --- |
| Object transformation | [`Partial<T>`](#partial-t), [`Optional<T, K>`](#optional-t-k), [`Required<T>`](#required-t), [`Readonly<T>`](#readonly-t), [`DeepPartial<T>`](#deeppartial-t) |
| Key selection | [`Pick<T, K>`](#pick-t-k), [`Omit<T, K>`](#omit-t-k) |
| Type-based selection | [`PickByType<T, Type>`](#pickbytype-t-type), [`OmitByType<T, Type>`](#omitbytype-t-type) |
| Key renaming | [`CamelCaseKeys<T>`](#camelcasekeys-t), [`SnakeCaseKeys<T>`](#snakecasekeys-t) |
| Field control | [`Require<T, K>`](#require-t-k), [`AtLeastOne<T, K>`](#atleastone-t-k) |
| Data extraction | [`DataOnly<T>`](#dataonly-t) |
| Key/value mapping | [`keyof`](#keyof), [`Record<K, T>`](#record-k-t) |
| Function types | [`ReturnType<T>`](#returntype-t), [`Parameters<T>`](#parameters-t) |
| Union manipulation | [`Exclude<T, U>`](#exclude-t-u), [`Extract<T, U>`](#extract-t-u) |
| Conditional types | [`T extends U ? X : Y`](#t-extends-u-x-y), [`infer R`](#infer-r) |

### Object transformation

#### `Partial<T>`

Makes every field optional, the caller can provide any subset.

<SignatureBlock label="Signature">Partial&lt;T&gt; = { [K in keyof T]?: T[K] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/partial.md -->

::: danger error
<!-- @include: ../.fragments/language/objects/type-utilities/partial-error.md -->
:::

#### `Optional<T, K>`

Keeps every field and makes only the selected keys optional. The existing one-argument form, `Optional<T>`, remains the spelling for `T?`.

<SignatureBlock label="Signature">Optional&lt;T, K extends keyof T&gt; = { [P in keyof T]: P extends K ? T[P]? : T[P] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/optional.md -->

#### `Required<T>`

Turns every optional field into a mandatory one, all fields must be provided.

<SignatureBlock label="Signature">Required&lt;T&gt; = { [K in keyof T]-?: T[K] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/required.md -->

#### `Readonly<T>`

Prevents field reassignment, the entire object becomes deeply immutable. It is one of six places the keyword appears; the [Readonly](/language/readonly) page covers them all.

<SignatureBlock label="Signature">Readonly&lt;T&gt; = { readonly [K in keyof T]: Readonly&lt;T[K]&gt; }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/readonly.md -->

#### `DeepPartial<T>`

Makes every field optional recursively, up to 5 levels deep. Ideal for partial update payloads (PATCH).

<SignatureBlock label="Signature">DeepPartial&lt;T&gt; = { [K in keyof T]?: DeepPartial&lt;T[K]&gt; }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/deep-partial.md -->

::: danger error
<!-- @include: ../.fragments/language/objects/type-utilities/deep-partial-error.md -->
:::

### Key selection

#### `Pick<T, K>`

Selects a subset of fields by key, the result has only the listed keys.

<SignatureBlock label="Signature">Pick&lt;T, K extends keyof T&gt; = { [P in K]: T[P] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/pick.md -->

#### `Omit<T, K>`

Removes a subset of fields by key, the result has everything except the listed keys.

<SignatureBlock label="Signature">Omit&lt;T, K extends keyof T&gt; = { [P in Exclude&lt;keyof T, K&gt;]: T[P] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/omit.md -->

### Value projections

`value.pick(keys)` and `value.omit(keys)` create a new object with the selected or remaining fields. Keys must be known at compile time, either in an array literal or in a variable with a finite literal-key type. The receiver is not mutated, field order follows the source type, and there is no runtime key lookup.

<!-- @include: ../.fragments/language/objects/type-utilities/optional.md -->

### Type-based selection

#### `PickByType<T, Type>`

Selects fields by their value type, keeps only the fields whose type matches `Type`.

<SignatureBlock label="Signature">PickByType&lt;T, Type&gt; = { [K in keyof T as T[K] extends Type ? K : never]: T[K] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/pick-by-type.md -->

::: danger error
<!-- @include: ../.fragments/language/objects/type-utilities/pick-by-type-error.md -->
:::

#### `OmitByType<T, Type>`

Removes fields by their value type, excludes all fields whose type matches `Type`.

<SignatureBlock label="Signature">OmitByType&lt;T, Type&gt; = { [K in keyof T as T[K] extends Type ? never : K]: T[K] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/omit-by-type.md -->

::: danger error
<!-- @include: ../.fragments/language/objects/type-utilities/omit-by-type-error.md -->
:::

### Key/value mapping

#### `keyof`

Extracts the keys of a type as a string literal union.

<SignatureBlock label="Signature">keyof T = string literal union of T's keys</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/keyof.md -->

#### `Record<K, T>`

`Record<K, T>` is a built-in type utility, like `Pick<T, K>` and `Omit<T, K>`. It is resolved by the compiler and does not create a runtime `Record` class. When `K` is a finite union of string literal keys, Tyrus produces an object type with one required field per key. When `K` is open, such as `string`, Tyrus resolves it to `Map<K, T>` for dynamic keys.

<SignatureBlock label="Signature">Record&lt;K extends string, T&gt; = required object fields for finite literal K; Map&lt;K, T&gt; for open K</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/record.md -->

### Key renaming

#### `CamelCaseKeys<T>`

Transforms all `snake_case` field names to `camelCase`. Essential for integrating with external APIs.

<SignatureBlock label="Signature">CamelCaseKeys&lt;T&gt; = { [K in camelCase&lt;keyof T&gt;]: T[snake_case&lt;K&gt;] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/camel-case-keys.md -->

::: danger error
<!-- @include: ../.fragments/language/objects/type-utilities/camel-case-keys-error.md -->
:::

#### `SnakeCaseKeys<T>`

Transforms all `camelCase` field names to `snake_case`. Useful for legacy API payloads and databases.

<SignatureBlock label="Signature">SnakeCaseKeys&lt;T&gt; = { [K in snake_case&lt;keyof T&gt;]: T[camelCase&lt;K&gt;] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/snake-case-keys.md -->

::: danger error
<!-- @include: ../.fragments/language/objects/type-utilities/snake-case-keys-error.md -->
:::

### Function types

#### `ReturnType<T>`

Infers the return type of a function type, extracts what the function produces.

<SignatureBlock label="Signature">ReturnType&lt;T extends (...args: any) =&gt; any&gt; = T's return type</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/returntype.md -->

#### `Parameters<T>`

Infers the parameter types of a function type as a tuple, extracts what the function expects.

<SignatureBlock label="Signature">Parameters&lt;T extends (...args: any) =&gt; any&gt; = T's parameter tuple</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/parameters.md -->

### Field control

#### `Require<T, K>`

Makes specific optional fields required. A noop if the field is already required.

<SignatureBlock label="Signature">Require&lt;T, K extends keyof T&gt; = T & { [P in K]-?: T[P] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/require.md -->

::: danger error
<!-- @include: ../.fragments/language/objects/type-utilities/require-error.md -->
:::

#### `AtLeastOne<T, K>`

Makes all specified fields optional but requires at least one of them to be present. Every key in `K` must be a property of `T`.

<SignatureBlock label="Signature">AtLeastOne&lt;T, K extends keyof T&gt; = { [P in keyof T]: P extends K ? T[P]? : T[P] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/at-least-one.md -->

::: danger error
<!-- @include: ../.fragments/language/objects/type-utilities/at-least-one-error.md -->
:::

### Data extraction

#### `DataOnly<T>`

Removes all function-typed fields, keeping only pure data properties. Ideal for API payloads.

<SignatureBlock label="Signature">DataOnly&lt;T&gt; = { [K in keyof T as T[K] extends Function ? never : K]: T[K] }</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/data-only.md -->

::: danger error
<!-- @include: ../.fragments/language/objects/type-utilities/data-only-error.md -->
:::

### Union manipulation

#### `Exclude<T, U>`

Removes members of `U` from `T`, keeps everything in `T` that is not in `U`.

<SignatureBlock label="Signature">Exclude&lt;T, U&gt; = T extends U ? never : T</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/exclude.md -->

#### `Extract<T, U>`

Keeps only members of `T` that also belong to `U`, computes the intersection.

<SignatureBlock label="Signature">Extract&lt;T, U&gt; = T extends U ? T : never</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/extract.md -->

### Conditional types

Conditional types choose between two types at compile time. The `extends` constraint selects the branch, and `infer` extracts a type from inside the matched structure.

#### `T extends U ? X : Y`

A conditional type resolves to `X` when `T` satisfies the constraint `U`, and to `Y` otherwise.

<SignatureBlock label="Signature">T extends U ? Then : Else</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/conditional.md -->

#### `infer R`

Binds a type variable inside the constraint of a conditional, when the constraint holds, the inferred type becomes the true branch.

<SignatureBlock label="Signature">T extends (arg: i32) =&gt; infer R ? R : never</SignatureBlock>

<!-- @include: ../.fragments/language/objects/type-utilities/infer.md -->

## Union Types

<!-- @include: ../.fragments/language/objects/union-types.md -->

Tagged unions narrow through literal discriminant checks. The compiler tracks positive and negative branches so each branch exposes the fields available for the narrowed variant set.

Unions also work with enums for type-safe event payloads:

<!-- @include: ../.fragments/language/objects/union-with-enums.md -->

## Algebraic enums

Algebraic enums model a closed set of variants, where each variant may have no payload, positional payload fields, or named payload fields. A `match` over an algebraic enum must be exhaustive unless it includes `_`; tuple and struct patterns bind the selected payload directly.

<!-- @include: ../.fragments/language/objects/algebraic-enums.md -->
