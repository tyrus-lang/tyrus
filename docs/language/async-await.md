---
title: Async and Await
description: Async functions, await expressions, Promise<T>, combinators, and cooperative cancellation in tyrus.
---

# Async and Await

tyrus supports executable `async` functions, `await` expressions, native `Promise<T>` values, and structured rejection handling.

## Async Functions

Declare an async function with the `async` keyword. It returns a `Promise<T>` that resolves to the inner type.

<!-- @include: ../.fragments/language/async-await/async-function.md -->

A `void` async function returns `Promise<void>` and does not carry a payload.

<!-- @include: ../.fragments/language/async-await/void-async.md -->

## Top-Level Await

A parameterless `async function main(): Promise<void>` can suspend at multiple top-level `await` statements.

<!-- @include: ../.fragments/language/async-await/top-level-await.md -->

## Arrow Async Functions

Async arrow functions follow the same syntax.

<!-- @include: ../.fragments/language/async-await/arrow-async.md -->

## Mutable Objects Across Await

A mutable object whose required stored fields are all `Copy` values can remain live across suspensions. Its `Copy` fields may be changed in place, and the whole binding may be replaced with another compatible object literal.

<!-- @include: ../.fragments/language/async-await/mutable-copy-object.md -->

Mutable objects with supported heap-owned fields may be replaced as a whole or updated field by field. Assignment to string, array, optional, and nested object fields evaluates and deeply clones the replacement before releasing the previous field, so self-derived assignments remain valid. Mutating string-array field methods such as `push`, `splice`, `fill`, and `sort` operate directly on the frame-owned field. Every recursively owned local carries an initialization bit, and generated type-specific drop glue releases only initialized values exactly once during replacement or frame destruction.

<!-- @include: ../.fragments/language/async-await/mutable-owned-object.md -->

## Nested Control Flow

Suspensions may appear through arbitrarily nested blocks, conditional branches, loops, switches, and structured error handling. The async state machine preserves active loop instances and lexical scopes between sibling suspension paths, so `break`, `continue`, branch-local bindings, and statements before or after each suspension retain their normal behavior. Both branches of a nested `if` may contain independent suspension chains with different lengths, payload types, and bindings; the condition is evaluated once.

<!-- @include: ../.fragments/language/async-await/nested-control-flow.md -->

## Error Handling

Use `try`/`catch`/`finally` to handle async rejections.

When a suspending `finally`, including one nested recursively inside loops and branches, is entered with a pending `return`, `break`, `continue`, or `throw`, the async frame preserves that completion until cleanup finishes. A completion produced inside `finally` replaces the pending one, and rejection from an awaited cleanup operation takes precedence. Each `finally` executes exactly once.

::: danger error
<!-- @include: ../.fragments/language/async-await/error-handling.md -->
:::

A `throw` inside an async function rejects its returned Promise.

::: danger error
<!-- @include: ../.fragments/language/async-await/throw-async.md -->
:::

## AggregateError

When multiple operations fail, `AggregateError` groups all rejection reasons.

::: danger error
<!-- @include: ../.fragments/language/async-await/aggregate-error.md -->
:::

## Promise.resolve and Promise.reject

Create resolved or rejected promises directly.

<SignatureBlock label="Signature">Promise.resolve&lt;T&gt;(value: T): Promise&lt;T&gt;</SignatureBlock>

<SignatureBlock label="Signature">Promise.reject&lt;T&gt;(reason: Error): Promise&lt;T&gt;</SignatureBlock>

<!-- @include: ../.fragments/language/async-await/promise-resolve-reject.md -->

## Promise.all

Wait for all promises to resolve. Rejects immediately if any promise rejects.

<SignatureBlock label="Signature">Promise.all&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;T[]&gt;</SignatureBlock>

<!-- @include: ../.fragments/language/async-await/promise-all.md -->

## Promise.allSettled

Wait for all promises to settle (resolve or reject). Never rejects.

<SignatureBlock label="Signature">Promise.allSettled&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;PromiseSettledResult&lt;T&gt;[]&gt;</SignatureBlock>

<!-- @include: ../.fragments/language/async-await/promise-all-settled.md -->

## Promise.race

Resolves or rejects with the first settled promise.

<SignatureBlock label="Signature">Promise.race&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;T&gt;</SignatureBlock>

<!-- @include: ../.fragments/language/async-await/promise-race.md -->

## Promise.any

Resolves with the first successful promise. Rejects only if all promises reject.

<SignatureBlock label="Signature">Promise.any&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;T&gt;</SignatureBlock>

<!-- @include: ../.fragments/language/async-await/promise-any.md -->

## Promise.withResolvers

Create a promise with external resolve and reject functions.

<SignatureBlock label="Signature">Promise.withResolvers&lt;T&gt;(): { promise: Promise&lt;T&gt;, resolve: (value: T) =&gt; void, reject: (reason: Error) =&gt; void }</SignatureBlock>

<!-- @include: ../.fragments/language/async-await/promise-with-resolvers.md -->

## then, catch, and finally

Chain promise continuations without `await`.

<!-- @include: ../.fragments/language/async-await/then-catch-finally.md -->

## Cooperative Cancellation

Use `cancel` from `tyrus:task` to cooperatively cancel a pending promise.

<!-- @include: ../.fragments/language/async-await/cancellation.md -->

## Sleep

Delay execution without blocking the executor thread.

<!-- @include: ../.fragments/language/async-await/sleep.md -->

## Promise Combinators in Practice

<!-- @include: ../.fragments/language/async-await/promise-combinators-practice.md -->

## Awaiting Once

A `Promise<T>` is consumed by `await`. The runtime holds one reference to it, and `await` takes that reference and releases it, so awaiting the same binding twice is a compile error (`E3006`) rather than a second read of a settled value:

<!-- @include: ../.fragments/language/async-await/await-once.md -->

Bind the awaited value and read the binding instead. The same rule applies to `JoinHandle<T>` and `.join()`.

## Error Examples

::: danger Async function must return Promise
An async function that returns a non-Promise type is a compile error:

<!-- @include: ../.fragments/language/async-await/err-bad-return.md -->

<!-- @include: ../.fragments/language/async-await/err-bad-return-output.md -->

:::

::: warning Discarded Promise
A Promise that is not awaited, returned, or stored produces a warning:

<!-- @include: ../.fragments/language/async-await/err-discarded-promise.md -->

<!-- @include: ../.fragments/language/async-await/err-discarded-promise-output.md -->

:::

::: danger await outside async
Using `await` outside an async function is a compile error:

<!-- @include: ../.fragments/language/async-await/err-outside-async.md -->

<!-- @include: ../.fragments/language/async-await/err-outside-async-output.md -->

:::
