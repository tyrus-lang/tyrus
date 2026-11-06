---
title: suffixArray
description: "Suffix array construction for strings from tyrus:algorithm."
---

# suffixArray

`suffixArray(text: string): usize[]` returns the starting index of every suffix of `text`, sorted lexicographically by comparing the suffix bytes. The result is a fresh array owned by the caller, with one entry per input character; its first element is the starting index of the smallest suffix. It is a free function of `tyrus:algorithm`, imported like any other name from that module.

## Function

| Signature | Returns | Description |
|-----------|---------|-------------|
| `suffixArray(text: string)` | `usize[]` | Starting index of every suffix, sorted lexicographically |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| construction | O(n² log n), direct suffix sorting |
| memory | O(n) |

## Examples

### banana

<!-- @include: ../../.fragments/modules/algorithm/suffix-array/banana.md -->

### empty and single character

<!-- @include: ../../.fragments/modules/algorithm/suffix-array/empty-and-single-character.md -->

### namespace import

<!-- @include: ../../.fragments/modules/algorithm/suffix-array/namespace-import.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/suffix-array/study-result.md -->

`sortRoundCount` counts bottom-up merge rounds, `comparisonCount` counts suffix comparisons, and `comparedByteCount` counts byte pairs inspected by those comparisons. Instrumentation retains O(n) additional storage and the same O(n² log n) worst-case time bound as direct suffix sorting.
