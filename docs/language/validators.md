---
title: Validator Decorations
description: Native validation annotations for interfaces, object types, function parameters, HTTP boundaries, and runtime checks.
---

# Validator Decorations

Validator Decorations are native validation annotations for tyrus. They can be attached to interface fields, object type fields and function parameters. The compiler checks that each validator exists, targets a compatible type and receives literal arguments of the expected kind.

## Example

<!-- @include: ../.fragments/language/validators/example.md -->

Built-in validators check literal values at compile time and run at runtime for dynamic values in decorated parameters, object literals and object updates.

## Validator Decorations in `.env` Files

Tyrus accepts Validator Decorations in `.env` files with or without a leading `#`. Use the commented form when another `.env` loader, such as Docker, also reads the file; Tyrus still parses the decoration while other loaders see a comment.

```dotenv
# @type(u16)
# @port()
PORT=8080
```

## Error Messages

Built-in validators let you override their default message with a string literal as the last argument they accept.

<!-- @include: ../.fragments/language/validators/error-messages.md -->

The same rule works on decorated function parameters.

<!-- @include: ../.fragments/language/validators/error-messages-parameter.md -->

::: tip Prefer user-facing custom messages at boundaries
Default messages are useful during development. Custom messages are better for API boundaries, CLI prompts, and validation failures that may be shown to users.
:::

## Validator Families

| Family | Examples |
| --- | --- |
| Numeric | [`@min`](#min), [`@max`](#max), [`@range`](#range), [`@positive`](#positive), [`@negative`](#negative), [`@nonNegative`](#nonnegative), [`@nonPositive`](#nonpositive), [`@multipleOf`](#multipleof), [`@powerOfTwo`](#poweroftwo), [`@integer`](#integer), [`@safeInteger`](#safeinteger), [`@port`](#port), [`@statusCode`](#statuscode) |
| String | [`@minLength`](#minlength), [`@maxLength`](#maxlength), [`@length`](#length), [`@notEmpty`](#notempty), [`@notBlank`](#notblank), [`@isTrimmed`](#istrimmed), [`@isLowercase`](#islowercase), [`@isUppercase`](#isuppercase), [`@startsWith`](#startswith), [`@endsWith`](#endswith), [`@includes`](#includes), [`@regex`](#regex) |
| Format | [`@email`](#email), [`@url`](#url), [`@uuid`](#uuid), [`@ip`](#ip), [`@ipv4`](#ipv4), [`@ipv6`](#ipv6), [`@e164`](#e164), [`@cep`](#cep), [`@cpf`](#cpf), [`@cnpj`](#cnpj) |
| Arrays and values | [`@minItems`](#minitems), [`@maxItems`](#maxitems), [`@lengthItems`](#lengthitems), [`@notEmptyItems`](#notemptyitems), [`@unique`](#unique), [`@in`](#in), [`@notIn`](#notin), [`@equal`](#equal), [`@notEqual`](#notequal), [`@notNull`](#notnull), [`@optional`](#optional), [`@required`](#required) |

## Validator Targets

A numeric validator is checked in the arithmetic of the type it decorates: an `i64` field is compared as an integer and an `f64` field as a float. A whole number past 2^53 therefore keeps its exact value, and the compile-time verdict always agrees with the check emitted for the same value.

`@port`, `@powerOfTwo`, and `@statusCode` target integer types only, because their rules have no reading on a fractional value. A port is not `8080.5`.

`@unique` compares elements by value, so it accepts arrays of numbers, strings, booleans, enums, tuples, object types, and named interfaces whose fields are themselves compared by value. An array of arrays is rejected at compile time: arrays are compared by address, so `[[1, 2], [1, 2]]` would be reported as unique.

## Validator Operation

Each validator family enforces specific constraints on the values it decorates. The compiler checks literal values at compile time and emits runtime checks for dynamic values. Below are examples for every validator family with their default error messages.

### Numeric

#### `@min`

Enforces that the value is at least the specified minimum.

<SignatureBlock label="Signature">@min(value: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/min.md -->

#### `@max`

Enforces that the value is at most the specified maximum.

<SignatureBlock label="Signature">@max(value: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/max.md -->

#### `@range`

Enforces that the value falls within the specified inclusive range.

<SignatureBlock label="Signature">@range(min: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, max: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/range.md -->

#### `@positive`

Enforces that the value is greater than zero.

<SignatureBlock label="Signature">@positive(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/positive.md -->

#### `@negative`

Enforces that the value is less than zero.

<SignatureBlock label="Signature">@negative(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/negative.md -->

#### `@nonNegative`

Enforces that the value is zero or greater.

<SignatureBlock label="Signature">@nonNegative(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/nonNegative.md -->

#### `@nonPositive`

Enforces that the value is zero or less.

<SignatureBlock label="Signature">@nonPositive(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/nonPositive.md -->

#### `@multipleOf`

Enforces that the value is a multiple of the given divisor.

<SignatureBlock label="Signature">@multipleOf(value: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/multipleOf.md -->

#### `@powerOfTwo`

Enforces that the value is a power of two.

<SignatureBlock label="Signature">@powerOfTwo(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/powerOfTwo.md -->

#### `@integer`

Enforces that a floating-point value has no fractional part.

<SignatureBlock label="Signature">@integer(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/integer.md -->

#### `@safeInteger`

Enforces that the integer value is within the safe integer range ±2⁵³.

<SignatureBlock label="Signature">@safeInteger(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/safeInteger.md -->

#### `@port`

Enforces that the value is a registered port number between 1024 and 49151.

<SignatureBlock label="Signature">@port(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/port.md -->

#### `@statusCode`

Enforces that the value is a valid HTTP status code between 100 and 599.

<SignatureBlock label="Signature">@statusCode(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/numeric/statusCode.md -->

### String

#### `@minLength`

Enforces that the string contains at least the specified number of characters.

<SignatureBlock label="Signature">@minLength(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/minLength.md -->

#### `@maxLength`

Enforces that the string contains at most the specified number of characters.

<SignatureBlock label="Signature">@maxLength(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/maxLength.md -->

#### `@length`

Enforces that the string has exactly the specified length.

<SignatureBlock label="Signature">@length(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/length.md -->

#### `@notEmpty`

Enforces that the string is not empty.

<SignatureBlock label="Signature">@notEmpty(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/notEmpty.md -->

#### `@notBlank`

Enforces that the string contains at least one non-whitespace character.

<SignatureBlock label="Signature">@notBlank(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/notBlank.md -->

#### `@isTrimmed`

Enforces that the string has no leading or trailing whitespace.

<SignatureBlock label="Signature">@isTrimmed(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/isTrimmed.md -->

#### `@isLowercase`

Enforces that the string contains only lowercase characters.

<SignatureBlock label="Signature">@isLowercase(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/isLowercase.md -->

#### `@isUppercase`

Enforces that the string contains only uppercase characters.

<SignatureBlock label="Signature">@isUppercase(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/isUppercase.md -->

#### `@startsWith`

Enforces that the string begins with the given prefix.

<SignatureBlock label="Signature">@startsWith(value: string, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/startsWith.md -->

#### `@endsWith`

Enforces that the string ends with the given suffix.

<SignatureBlock label="Signature">@endsWith(value: string, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/endsWith.md -->

#### `@includes`

Enforces that the string contains the given substring.

<SignatureBlock label="Signature">@includes(value: string, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/includes.md -->

#### `@regex`

Enforces that the string matches the given regular expression.

<SignatureBlock label="Signature">@regex(value: Regex, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/string/regex.md -->

### Format

#### `@email`

Enforces that the value is a valid e-mail address.

<SignatureBlock label="Signature">@email(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/format/email.md -->

#### `@url`

Enforces that the value is a valid URL.

<SignatureBlock label="Signature">@url(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/format/url.md -->

#### `@uuid`

Enforces that the value is a valid UUID.

<SignatureBlock label="Signature">@uuid(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/format/uuid.md -->

#### `@ip`

Enforces that the value is a valid IP address (v4 or v6).

<SignatureBlock label="Signature">@ip(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/format/ip.md -->

#### `@ipv4`

Enforces that the value is a valid IPv4 address.

<SignatureBlock label="Signature">@ipv4(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/format/ipv4.md -->

#### `@ipv6`

Enforces that the value is a valid IPv6 address.

<SignatureBlock label="Signature">@ipv6(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/format/ipv6.md -->

#### `@e164`

Enforces that the value is a valid E.164 phone number.

<SignatureBlock label="Signature">@e164(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/format/e164.md -->

#### `@cep`

Enforces that the value is a valid Brazilian postal code (CEP).

<SignatureBlock label="Signature">@cep(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/format/cep.md -->

#### `@cpf`

Enforces that the value is a valid Brazilian CPF.

<SignatureBlock label="Signature">@cpf(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/format/cpf.md -->

#### `@cnpj`

Enforces that the value is a valid Brazilian CNPJ.

<SignatureBlock label="Signature">@cnpj(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/format/cnpj.md -->

### Array

#### `@minItems`

Enforces that the array contains at least the specified number of items.

<SignatureBlock label="Signature">@minItems(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/array/minItems.md -->

#### `@maxItems`

Enforces that the array contains at most the specified number of items.

<SignatureBlock label="Signature">@maxItems(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/array/maxItems.md -->

#### `@lengthItems`

Enforces that the array has exactly the specified length.

<SignatureBlock label="Signature">@lengthItems(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/array/lengthItems.md -->

#### `@notEmptyItems`

Enforces that the array is not empty.

<SignatureBlock label="Signature">@notEmptyItems(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/array/notEmptyItems.md -->

#### `@unique`

Enforces that all items in the array are distinct.

<SignatureBlock label="Signature">@unique(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/array/unique.md -->

### Equality and membership

#### `@equal`

Enforces that the value equals the given literal.

<SignatureBlock label="Signature">@equal(value: T, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/equality-membership/equal.md -->

#### `@notEqual`

Enforces that the value differs from the given literal.

<SignatureBlock label="Signature">@notEqual(value: T, message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/equality-membership/notEqual.md -->

#### `@in`

Enforces that the value is one of the specified options.

<SignatureBlock label="Signature">@in(value: T[], message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/equality-membership/in.md -->

#### `@notIn`

Enforces that the value is not one of the specified options.

<SignatureBlock label="Signature">@notIn(value: T[], message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/equality-membership/notIn.md -->

### Nullability and presence

#### `@notNull`

Enforces that an optional target carries a value.

<SignatureBlock label="Signature">@notNull(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/nullability-presence/notNull.md -->

#### `@optional`

Marks a field or parameter as explicitly optional.

<SignatureBlock label="Signature">@optional(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/nullability-presence/optional.md -->

#### `@required`

Enforces that the target has a value.

<SignatureBlock label="Signature">@required(message?: string)</SignatureBlock>

<!-- @include: ../.fragments/language/validators/nullability-presence/required.md -->

Validation is applied automatically when typed objects are created or updated and when decorated functions are called. Route-schema behavior belongs to the HTTP roadmap and is not part of this language-level mechanism.

## Optional Targets

A validator placed on an optional target applies to the value the optional carries, so it only runs when a value is present. An absent optional passes every validator except `@notNull` and `@required`, which test presence itself and therefore apply to the optional rather than to its payload.

<!-- @include: ../.fragments/language/validators/optional-targets.md -->

Neither the omitted `email` field nor the omitted `age` argument fails validation: there is no value to check. Supplying an invalid value makes the same validators fail exactly as they would on a required target.

## Recoverable Runtime Validation

Dynamic validation failures throw `ValidationError`; they do not stop the application when handled. `ValidationError.issues` is always present and contains at least one read-only `ValidationIssue`, so the same handling works whether one rule or several rules fail. A decorated function implicitly declares this effect. An intermediate function must catch it or propagate it with `throws ValidationError`, while `main` must handle every reachable error and cannot declare `throws`.

Each issue target is a deterministic logical path. Nested fields use dot notation such as `profile.address.zipCode`, and array elements include their zero-based index such as `members[0].address.zipCode`.

The path is rooted at the value the operation validates, not at some outermost type. Building a nested object into its own variable first makes that construction a validation operation of its own, so its issues are rooted there: the target is `zipCode`, not `members[0].address.zipCode`.

One validation operation collects every failed rule before throwing: an object creation or update collects issues across all decorated fields, and a decorated function call collects issues across all decorated parameters. Issues preserve declaration and evaluation order. Separate operations throw separate `ValidationError` values and are never merged.

<!-- @include: ../.fragments/language/validators/recoverable-runtime.md -->

`ValidationIssue` is normally inferred when an item is read from `error.issues`; developers do not construct or throw it. Passing an issue to a read-only helper does not require writing `&`.

Reading and iterating `error.issues` inside the `catch` is the usual pattern. A direct move or return of `error.issues` is also safe: ownership follows the returned array instead of being released with the caught error.

<!-- @include: ../.fragments/language/validators/print-issue.md -->

Typed `.env` validation is the exception: invalid startup configuration is fatal because the application cannot start with an invalid environment.

`@in` and `@notIn` accept an inline homogeneous array or a module-level `const` array that can be fully resolved at compile time. The array may contain enum members. Runtime variables, `let` bindings, function calls and API values are rejected because decoration arguments are compile-time metadata. Typed `.env` schemas continue to use inline arrays because `.env` files have no tyrus symbol or import scope.

<!-- @include: ../.fragments/language/validators/in-not-in.md -->

## Regex Validators

<!-- @include: ../.fragments/language/validators/regex.md -->

`@regex` uses the native `Regex` type. Supported flags are `i`, `m`, `s` and `x`; look-around and backreferences are intentionally not supported.
