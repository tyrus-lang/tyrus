---
title: Deque
description: "Double-ended queue data structure from tyrus:algorithm."
---

# Deque

`Deque<T>` implements a double-ended queue with amortized O(1) insertion and removal at both ends.

## Construction

| Syntax | Description |
|--------|-------------|
| `Deque<T>()` | Create a new empty deque |

The type parameter `T` determines the element type. `T` can be a scalar value type (`boolean`, `char`, integer, or float); heap-owned values such as `string` are not supported.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `pushFront` | `(T) => void` | `void` | Add an element to the front |
| `pushBack` | `(T) => void` | `void` | Add an element to the back |
| `popFront` | `() => T?` | `T?` | Remove and return the front element, or empty if empty |
| `popBack` | `() => T?` | `T?` | Remove and return the back element, or empty if empty |
| `peekFront` | `() => T?` | `T?` | Return the front element without removing it, or empty if empty |
| `peekBack` | `() => T?` | `T?` | Return the back element without removing it, or empty if empty |
| `len` | `() => usize` | `usize` | Number of elements |
| `isEmpty` | `() => boolean` | `boolean` | Whether the deque has no elements |
| `clear` | `() => void` | `void` | Remove all elements |

`popFront`, `popBack`, `peekFront`, and `peekBack` return `T?`, call with a presence check or default value.

The native handle is released automatically when the binding goes out of scope.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `pushFront` | O(1) |
| `pushBack` | Amortized O(1) |
| `popFront` | O(1) |
| `popBack` | O(1) |
| `peekFront` | O(1) |
| `peekBack` | O(1) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(1) |

## Examples

### Construction

<!-- @include: ../../.fragments/modules/algorithm/deque/construction.md -->

### pushFront and pushBack

<!-- @include: ../../.fragments/modules/algorithm/deque/pushfront-and-pushback.md -->

### popFront

<!-- @include: ../../.fragments/modules/algorithm/deque/popfront.md -->

### popBack

<!-- @include: ../../.fragments/modules/algorithm/deque/popback.md -->

### peekFront and peekBack

<!-- @include: ../../.fragments/modules/algorithm/deque/peekfront-and-peekback.md -->

### len

<!-- @include: ../../.fragments/modules/algorithm/deque/len.md -->

### isEmpty

<!-- @include: ../../.fragments/modules/algorithm/deque/isempty.md -->

### clear

<!-- @include: ../../.fragments/modules/algorithm/deque/clear.md -->

### Mixed pushFront and popBack (queue behavior)

<!-- @include: ../../.fragments/modules/algorithm/deque/mixed-pushfront-and-popback-queue-behavior.md -->

