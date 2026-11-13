---
title: LinkedList
description: "Singly linked list data structure from tyrus:algorithm."
---

# LinkedList

`LinkedList<T>` implements a singly linked list with O(1) insertion and removal at the front.

## Construction

| Syntax | Description |
|--------|-------------|
| `LinkedList<T>()` | Create a new empty linked list |
| `LinkedList.fromArray(arr)` | Create a linked list with the elements of `arr` in order |

The type parameter `T` determines the element type. `T` can be a scalar value type (`boolean`, `char`, integer, or float); heap-owned values such as `string` are not supported.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `pushFront` | `(T) => void` | `void` | Add an element to the front |
| `pushBack` | `(T) => void` | `void` | Add an element to the back |
| `popFront` | `() => T?` | `T?` | Remove and return the front element, or empty if empty |
| `popBack` | `() => T?` | `T?` | Remove and return the back element, or empty if empty |
| `front` | `() => T?` | `T?` | Return the front element without removing it, or empty if empty |
| `back` | `() => T?` | `T?` | Return the back element without removing it, or empty if empty |
| `len` | `() => usize` | `usize` | Number of elements |
| `isEmpty` | `() => boolean` | `boolean` | Whether the list has no elements |
| `clear` | `() => void` | `void` | Remove all elements |

`popFront`, `popBack`, `front`, and `back` return `T?`, call with a presence check or default value.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `pushFront` | O(1) |
| `pushBack` | O(n) |
| `popFront` | O(1) |
| `popBack` | O(n) |
| `front` | O(1) |
| `back` | O(n) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n) |

## Examples

### Construction

<!-- @include: ../../.fragments/modules/algorithm/linked-list/construction.md -->

### pushFront and pushBack

<!-- @include: ../../.fragments/modules/algorithm/linked-list/pushfront-and-pushback.md -->

### front and back

<!-- @include: ../../.fragments/modules/algorithm/linked-list/front-and-back.md -->

### popFront

<!-- @include: ../../.fragments/modules/algorithm/linked-list/popfront.md -->

### popBack

<!-- @include: ../../.fragments/modules/algorithm/linked-list/popback.md -->

### len

<!-- @include: ../../.fragments/modules/algorithm/linked-list/len.md -->

### isEmpty

<!-- @include: ../../.fragments/modules/algorithm/linked-list/isempty.md -->

### clear

<!-- @include: ../../.fragments/modules/algorithm/linked-list/clear.md -->

### fromArray

<!-- @include: ../../.fragments/modules/algorithm/linked-list/fromarray.md -->
