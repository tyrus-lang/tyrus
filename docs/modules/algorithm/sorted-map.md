---
title: SortedMap
description: "Ordered key-value map from tyrus:algorithm."
---

# SortedMap

`SortedMap<K, V>` is an ordered key → value map backed by a balanced binary search tree. It keeps every entry sorted by key. The native handle is released automatically when the binding goes out of scope.

## Construction

| Syntax | Description |
|--------|-------------|
| `SortedMap<K, V>()` | Create a new empty ordered map |

`K` can be any integer type, any float type, `boolean`, or `string`. `V` can be any type.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `set` | `(K, V) => void` | `void` | Insert or replace the entry for `key` |
| `get` | `(K) => V?` | `V?` | Value for `key`, or no value when absent |
| `has` | `(K) => boolean` | `boolean` | Whether `key` is present |
| `delete` | `(K) => boolean` | `boolean` | Remove the entry for `key`; returns `true` when it was present |
| `min` | `() => K?` | `K?` | Smallest key, or no value when empty |
| `max` | `() => K?` | `K?` | Largest key, or no value when empty |
| `len` | `() => usize` | `usize` | Number of entries |
| `isEmpty` | `() => boolean` | `boolean` | Whether the map is empty |
| `clear` | `() => void` | `void` | Remove all entries |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `set` | O(log n) |
| `get` / `has` | O(log n) |
| `delete` | O(log n) |
| `min` / `max` | O(log n) |
| `len` / `isEmpty` | O(1) |
| `clear` | O(n) |

The tree stays balanced, so every lookup, insertion and deletion is logarithmic regardless of insertion order.

## Examples

### Complete example

<!-- @include: ../../.fragments/modules/algorithm/sorted-map/complete-example.md -->

### String keys

<!-- @include: ../../.fragments/modules/algorithm/sorted-map/string-keys.md -->
