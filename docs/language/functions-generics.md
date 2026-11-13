---
title: Functions, Closures, and Generics
description: Arrow functions, function values, closures, generic functions, constraints, and monomorphization.
---

# Functions, Closures, and Generics

Tyrus supports arrow functions and native function values.

## Functions

<!-- @include: ../.fragments/language/functions-generics/functions.md -->

Function signatures include parameter and return types. Expression bodies return implicitly. A non-`void` function must return a value on every reachable path.

## Function Values

<!-- @include: ../.fragments/language/functions-generics/function-values.md -->

Function values can be stored and called. They may capture lexical values, subject to ownership and lifetime rules.

A named function is itself a first-class value: it can be passed as an argument, aliased to another binding, and used as an array callback. A top-level `const name = (…) => …` declares a named function, so the same applies to it.

<!-- @include: ../.fragments/language/functions-generics/function-values-passing.md -->

A function can also return a function. The returned value keeps its function type, so a binding that receives it stays callable without an annotation.

<!-- @include: ../.fragments/language/functions-generics/make-adder.md -->

The returned function can also be called directly, without a binding in between. The callee is then an expression rather than a name, so it carries no signature to look up: its type is the whole contract, and calling something that is not a function value is `E0618` with the range on the callee.

<!-- @include: ../.fragments/language/functions-generics/immediate-call.md -->

::: warning Capture across two levels is not supported yet
A closure that captures a binding from two levels out, as in `(a) => (b) => (c) => a + b + c`, is rejected with `E0435`, anchored on the mention that reaches too far. A closure can only capture from the function directly around it.
:::


## Mutable Parameters

<!-- @include: ../.fragments/language/functions-generics/mutable-parameters.md -->

`mut` allows reassignment inside the function body without making the caller binding mutable.

## Rest Parameters

<!-- @include: ../.fragments/language/functions-generics/rest-parameters.md -->

A rest parameter (`...name: T[]`) must be the last parameter and collects the trailing arguments into an array. It accepts individual arguments, a `...spread` of an existing array, no arguments at all, and any combination alongside leading fixed parameters. Individual arguments are checked against the element type; a spread argument is checked against the whole array type.

## Closures

<!-- @include: ../.fragments/language/functions-generics/closures.md -->

Function values use a native closure representation with a function pointer and an environment pointer. Synchronous functional-array callbacks can capture lexical values without transferring ownership of the captured resource.

### What a Capture Sees

A closure that never leaves the function that declared it captures the bindings themselves. Reads see the current value, writes reach the binding outside, and state accumulates across calls:

<!-- @include: ../.fragments/language/functions-generics/confined-capture.md -->

A closure only stays inside its function when every mention of it is a call. Returning it, passing it as an argument, aliasing it to another name, or storing it anywhere hands its value to something that may outlive the function, and then the bindings it captured are gone before it runs.

Such a closure captures a *copy* instead, taken when the closure is built. Reading a copy is well defined. Writing one is not useful: the write lands in the copy, and nothing outside can ever observe it. That is `E0436`, anchored on the write itself:

<!-- @include: ../.fragments/language/functions-generics/escaping-capture.md -->

::: tip
The fix is to return the value instead of writing it, or to hold the shared state in a [`Mutex`](../modules/task/mutex.md) that both sides can reach. The second is what a task started with [`spawn`](../modules/task/spawn.md) needs, and the same rule is what keeps a task from sharing mutable state with the code that started it.
:::

## Generics

<!-- @include: ../.fragments/language/functions-generics/generics.md -->

Generic functions use native monomorphization: the compiler emits one LLVM specialization per concrete type combination. Generic substitution works through arrays, references, optionals and function types.
