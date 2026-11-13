---
title: Custom Validator
description: User-defined validators for decorated interface fields and function parameters.
---

# Custom Validator

Custom validators let an application define validation rules without adding new built-in validators. They can be used on interface fields and function parameters, and run synchronously when the decorated value is checked.

## Declaration

Declare a validator with `validator function` or `validator const`. Its first parameter must be explicitly typed as `value`; optional configuration parameters follow it and receive the positional arguments written in the decorator. The function returns the `Validator` union. A successful result is `{ isValid: true }`; a failure returns `{ isValid: false, message: "..." }`.

The compiler supplies the decorated field or parameter value as `value`. For example, `@hasPrefix("user_")` calls the validator with the value first and `"user_"` as its configuration argument. Use empty parentheses for validators without configuration, such as `@hasNoSpaces()`.

## Example

<!-- @include: ../.fragments/language/validators/custom-example.md -->

## Returned Messages

A custom validator chooses its failure message by returning `{ isValid: false, message }`. That message is included in the resulting `ValidationIssue`; the value being checked is not inserted automatically. A failed check throws `ValidationError`, so the enclosing function must catch it or declare `throws ValidationError`.
