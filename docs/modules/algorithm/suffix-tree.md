---
title: suffixTreeUkkonen
description: "Suffix tree construction for strings from tyrus:algorithm."
---

# suffixTreeUkkonen

`suffixTreeUkkonen(text: string): SuffixTree` builds a suffix tree over `text` using Ukkonen's linear-time algorithm. The returned `SuffixTree` is an owned handle of `tyrus:algorithm`; it exposes `getSuffixArray()`, which yields the starting index of every suffix of `text`, sorted lexicographically by comparing the suffix bytes, as a fresh array owned by the caller with one entry per input character.

## Construction

| Syntax | Description |
|--------|-------------|
| `suffixTreeUkkonen(text: string)` | Build a suffix tree over `text` |

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `getSuffixArray` | `() => usize[]` | `usize[]` | Starting index of every suffix, sorted lexicographically |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| construction | O(n), Ukkonen's algorithm |
| `getSuffixArray` | O(n), single traversal of the tree |
| memory | O(n) |

## Examples

### banana

<!-- @include: ../../.fragments/modules/algorithm/suffix-tree/banana.md -->

### annotated type

<!-- @include: ../../.fragments/modules/algorithm/suffix-tree/annotated-type.md -->

### empty and single character

<!-- @include: ../../.fragments/modules/algorithm/suffix-tree/empty-and-single-character.md -->

### namespace import

<!-- @include: ../../.fragments/modules/algorithm/suffix-tree/namespace-import.md -->
