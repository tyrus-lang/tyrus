---
title: BloomFilter
description: "Probabilistic set-membership structure with a configurable false-positive rate from tyrus:algorithm."
---

# BloomFilter

`BloomFilter` implements a probabilistic set-membership structure for `string` keys. A `BloomFilter(k, m)` stores keys across `m` bits using `k` independent hash functions: `contains` never reports a false negative, but may report a false positive with a probability that shrinks as the number of bits per key grows. It is non-generic: both constructor arguments are `usize`.

## Construction

| Syntax | Description |
|--------|-------------|
| `BloomFilter(k, m)` | Create a new empty filter with `k` hash functions over `m` bits |

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `insert` | `(key: string) => void` | `void` | Add `key` to the filter |
| `contains` | `(key: string) => boolean` | `boolean` | Whether `key` may have been inserted; never a false negative |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `insert` | O(k) |
| `contains` | O(k) |
| `BloomFilter(k, m)` | O(m / 64 + k) |

## Examples

### insert

<!-- @include: ../../.fragments/modules/algorithm/bloom-filter/insert.md -->

### contains

<!-- @include: ../../.fragments/modules/algorithm/bloom-filter/contains.md -->

The hashing is deterministic, so the misses above are guaranteed, not probabilistic. `contains` still allows false positives in general: once the filter is dense, an absent key may share all `k` bit positions with inserted keys.
