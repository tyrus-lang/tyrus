---
title: Set
description: Set collections and their native operations.
---

# Set


## Native methods and properties

| Member | Signature | Behavior |
| --- | --- | --- |
| `size` | `usize` | Number of distinct values. |
| `add` | `add(value: T): Set<T>` | Inserts a value if it is not already present and returns the set. |
| `has` | `has(value: T): boolean` | Checks whether a value is present. |
| `delete` | `delete(value: T): boolean` | Removes a value and reports whether it was present. |
| `values` | `values(): T[]` | Returns the current values as an array. |

A set can also be traversed directly with `for...of`. Adding an existing value does not create a duplicate.

<!-- @include: ../.fragments/language/collections/set.md -->
