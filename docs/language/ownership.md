---
title: Ownership and Borrowing
description: Move semantics, Copy values, immutable and mutable borrows, lifetimes, nullable values, and deferred initialization.
---

# Ownership and Borrowing

Tyrus uses concise syntax and enforces ownership rules at compile time.

## Move and Copy

Non-Copy values such as `string` move on assignment. The original binding becomes invalid after the move.

::: danger error
<!-- @include: ../.fragments/language/ownership/move-copy.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/ownership/moved-value-diagnostic.md -->
:::

Primitive values such as `i32`, `i64`, `f32`, `f64`, and `boolean` are Copy and remain usable after assignment.

<!-- @include: ../.fragments/language/ownership/copy-primitives.md -->

Function calls pass read-only arguments without consuming ordinary non-Copy caller bindings. Explicit references are available when an API needs to expose borrowing.

## Borrowing

<!-- @include: ../.fragments/language/ownership/borrowing.md -->


A borrow may take a part of a value: `&basket.label` borrows the field. The reference reads through `basket`, so `basket` is what stays borrowed, and assigning to it while the reference is live is refused. Borrowing two different fields at once is refused for the same reason, which is stricter than it has to be and never wrong.

<!-- @include: ../.fragments/language/ownership/borrowing-a-field.md -->
You may have many immutable borrows or one mutable borrow, never both at the same time. Temporary borrows end at the end of the statement. A stored borrow ends at the last point its binding is mentioned, not at the closing brace: once nothing reads the reference again, the value it pointed at is free to be assigned, borrowed mutably, or moved, even though the binding is still in scope. A mention inside a loop body or captured by a closure counts, and keeps the borrow alive for as long as that construct can run. A mention inside a `defer` or `errdefer` body keeps it alive to the closing brace, wherever the finalizer is written, because that is where the finalizer runs. References cannot outlive the value they point to.

Inside an `async` function a borrow behaves the same way, with one added rule: it may not still be live at an `await`. Holding one across a suspension is rejected, "borrow `b` of `a` crosses `await`", which is also what makes borrows cheap there, since the state machine keeps the borrowed bytes in its frame and releases none of them.

## Non-Lexical Lifetimes (NLL)

Tyrus implements Non-Lexical Lifetimes (NLL). Instead of keeping borrows alive until the closing curly brace of their lexical block (`}`), the borrow checker builds a Control Flow Graph (CFG) and uses backward dataflow liveness analysis to release loans immediately after their last actual use.

### Mutable Borrow Followed by Mutation

A mutable borrow releases the original variable as soon as the reference is no longer read:

<!-- @include: ../.fragments/language/ownership/nll-mutable-borrow.md -->

Without NLL, `r` would remain live until the enclosing scope closed, rejecting `x += 1`.

### Borrows Inside Conditional Branches

Loans taken inside an `if` branch expire when that branch finishes or ceases to use them, freeing the outer value:

<!-- @include: ../.fragments/language/ownership/nll-conditional-branches.md -->

### Move After Borrow

Once a borrowed reference is no longer accessed, the owned value can be moved or consumed:

<!-- @include: ../.fragments/language/ownership/nll-move-after-borrow.md -->

### Reassignment of References

Reassigning a mutable reference to a new target immediately releases the borrow on the previous target:

<!-- @include: ../.fragments/language/ownership/nll-reassignment.md -->

### Escape Prevention

NLL only shortens lifetimes to where references are actually needed; it never allows a reference to outlive its owner. Returning or storing a reference to a local variable that leaves scope is strictly rejected:

::: danger error
<!-- @include: ../.fragments/language/ownership/nll-escape-prevention.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/ownership/nll-escape-prevention-diagnostic.md -->
:::

## Nullable Values

tyrus has no `undefined`, and user code should not use `null` as a normal initializer. Nullable values use `T?`, optional fields, optional parameters, external boundaries, narrowing and `??`.

<!-- @include: ../.fragments/language/ownership/nullable-values.md -->

Forced unwrap (`value!`) is intentionally discouraged and diagnosed by language tooling; prefer explicit narrowing or fallback values.

## Deferred Initialization

<!-- @include: ../.fragments/language/ownership/deferred-init.md -->

The compiler runs definite-assignment analysis and rejects reading a `let x: T;` binding before every path assigns it. `const` bindings must be initialized where they are declared.
