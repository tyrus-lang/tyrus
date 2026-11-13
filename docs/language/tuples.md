---
title: Tuples
description: Fixed-size tuple types, indexing, and destructuring.
---

# Tuples


## Native tuple operations

| Operation | Behavior |
| --- | --- |
| `length` | Read-only compile-time tuple length. |
| `tuple[index]` | Reads or writes an element using an integer literal index; the element type is determined by that position. |
| Destructuring | Extracts tuple elements into local bindings; `_` discards an element. |
| Mutating array methods | Methods that change elements or length are rejected because a tuple has a fixed layout. |

Methods that return a new array can be used where their signatures accept a sequence; see [Array](./arrays).
<!-- @include: ../.fragments/language/collections/tuples.md -->

Tuples lower to native structs, while arrays are dynamic collections. Tuple indexes must be integer literals, so out-of-range access is a compile error.

A wildcard `_` in a destructuring position discards that element: it is still extracted and released like any other local, but declares no name the rest of the scope can read back, so a later `_` in the same pattern never collides with an earlier one. Reading `_` afterward is an ordinary undeclared-identifier error, the same as any other name that was never declared.
