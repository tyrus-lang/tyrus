---
title: Counter
description: "Frequency counter data structure from tyrus:algorithm."
---

# Counter

`Counter<T>` counts scalar values and preserves their first-insertion order when frequencies tie.

## Construction

`Counter<T>()` creates an empty counter. `T` must be a scalar value type: `boolean`, `char`, an integer, or a floating-point type. Heap-owned values such as `string` are not supported.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `add` | `(T, i32?) => void` | `void` | Increment an item by a positive count; the default is 1. |
| `remove` | `(T, i32?) => void` | `void` | Decrement an item by a positive count; missing or exhausted items are absent. |
| `count` | `(T) => i32` | `i32` | Return the count, or 0 when absent. |
| `mostCommon` | `(usize) => [T, i32][]` | `[T, i32][]` | Return up to that many `[item, count]` pairs ordered by descending count and stable on ties. |
| `total` | `() => i32` | `i32` | Return the sum of all counts. |
| `unique` | `() => usize` | `usize` | Return the number of present items. |
| `isEmpty` | `() => boolean` | `boolean` | Return whether the counter is empty. |
| `clear` | `() => void` | `void` | Remove all items. |

`add`, `remove`, and `total` abort with a runtime error if a positive `i32` count cannot be represented.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `add`, `remove`, `count` | O(1) average |
| `mostCommon(k)` | O(n log n) |
| `total`, `clear` | O(n) |
| `unique`, `isEmpty` | O(1) |

## Example

<!-- @include: ../../.fragments/modules/algorithm/counter/example.md -->
