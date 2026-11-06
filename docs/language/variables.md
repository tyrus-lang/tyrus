---
title: Variables
description: Variable declarations with let and const, mutability, deep immutability, and uninitialized optional bindings.
---

# Variables

Tyrus provides two declaration keywords for bindings: `const` and `let`. Both enforce strict lexical scoping and participate in static type checking, but they differ in mutability, rebinding rules, and initialization requirements.

## const vs let

The choice between `const` and `let` establishes whether a binding and its underlying data can be mutated.

| Feature | `const` | `let` |
|---|---|---|
| Reassignment | Prohibited at compile time | Allowed |
| Deep Immutability | Enforced recursively on objects and arrays | Not enforced (properties and elements are mutable) |
| Initializer Required | Always required | Optional (`let x: i32;` is allowed) |
| Shadowing | Prohibited | Prohibited |

## Immutable Bindings with const

`const` declares an immutable binding whose value cannot be reassigned after initialization. Use `const` by default whenever a variable does not require reassignment.

<!-- @include: ../.fragments/language/variables/const-valid.md -->

Attempting to assign a new value to a `const` binding produces a compile-time error.

::: danger error
<!-- @include: ../.fragments/language/variables/const-error.md -->
:::

## Deep Immutability with const

In Tyrus, `const` guarantees deep immutability: it prevents rebinding and prohibits modifying the properties of an object or the elements of an array declared with `const`.

<!-- @include: ../.fragments/language/variables/deep-immutability-valid.md -->

Mutating an object field or array element through a `const` reference is rejected by the compiler.

::: danger error
<!-- @include: ../.fragments/language/variables/deep-immutability-error.md -->
:::

## Mutable Bindings with let

`let` declares a mutable binding. It allows both variable reassignment and in-place mutation of composite structures such as arrays and objects.

<!-- @include: ../.fragments/language/variables/let-valid.md -->

Reassignment must still conform to the statically declared or inferred type of the variable.

::: danger error
<!-- @include: ../.fragments/language/variables/let-error.md -->
:::

## Uninitialized let Bindings

A variable declared with `let` can be declared without an immediate initializer, even for non-optional types such as `let number: i32;`. Optional bindings (`T?`) start empty and can be evaluated safely with null-coalescing (`??`), while non-optional bindings must be assigned before they can be read.

<!-- @include: ../.fragments/language/variables/uninitialized-valid.md -->

Reading an uninitialized non-optional variable before assignment is an error, and declaring a `const` without an initializer is prohibited.

::: danger error
<!-- @include: ../.fragments/language/variables/uninitialized-error.md -->
:::

## No Shadowing Allowed

Tyrus does not permit variable shadowing within the same scope. To update a value, declare a mutable binding with `let` and reassign it instead of redeclaring the variable.

<!-- @include: ../.fragments/language/variables/shadowing-valid.md -->

Attempting to redeclare a variable with `let` or `const` in the same scope is rejected by the compiler.

::: danger error
<!-- @include: ../.fragments/language/variables/shadowing-error.md -->
:::

## Type Inference and Annotations

Variable types can be explicitly annotated or inferred automatically from the initializer expression.

<!-- @include: ../.fragments/language/variables/type-annotation-valid.md -->

When an explicit type annotation is supplied, the assigned value must satisfy that type constraint.

::: danger error
<!-- @include: ../.fragments/language/variables/type-annotation-error.md -->
:::
