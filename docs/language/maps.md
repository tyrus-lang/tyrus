---
title: Map
description: Map collections and their native methods.
---

# Map




## Native methods and properties

| Member | Signature | Behavior |
| --- | --- | --- |
| `size` | `usize` | Number of key-value pairs. |
| `set` | `set(key: K, value: V): Map<K, V>` | Inserts or replaces a value for a key and returns the map. |
| `get` | `get(key: K): V?` | Returns the value, or absence when the key is missing. |
| `getOrInsert` | `getOrInsert(key: K, fallback: V): V` | Returns a mutable reference to the stored value, inserting the fallback only when the key is missing. |
| `has` | `has(key: K): boolean` | Checks whether a key is present. |
| `delete` | `delete(key: K): boolean` | Removes a key and reports whether it was present. |
| `keys` | `keys(): K[]` | Returns the keys as an array. |
| `values` | `values(): V[]` | Returns the values as an array. |
| `entries` | `entries(): [K, V][]` | Returns key-value pairs as an array; it can also be traversed with `for...of`. |

## Map

<!-- @include: ../.fragments/language/collections/map.md -->

`getOrInsert(key, value)` returns a mutable reference to the value for `key`. The fallback expression is lazy and runs only when the key is absent.
