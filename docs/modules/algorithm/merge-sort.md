---
title: mergeSort
description: "Stable merge sort from tyrus:algorithm."
---

# mergeSort

`mergeSort(values)` returns a stable ascending copy of an `i32[]`. Equal values retain their input order when used with richer values through a future comparator-aware API.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `mergeSort(values: i32[])` | `i32[]` | O(n log n) time, O(n) memory |

<!-- @include: ../../.fragments/modules/algorithm/merge-sort/mergesort.md -->

## Study result

| Export | Type | Purpose |
|--------|------|---------|
| `mergeSortInfo(values: i32[])` | Function | Returns an independent `SortInfo` snapshot and operation metrics |
| `SortInfo` | Data interface | Holds the original and sorted arrays, comparison count, and swap count |

`SortInfo` exposes `originalValues`, `sortedValues`, `comparisonCount`, and `swapCount`. Non-comparison sorts report zero comparisons and zero swaps because they do not compare values or exchange array elements. The input array is unchanged.
### Interface example

```tyrus
interface SortInfo {
    originalValues: i32[];
    sortedValues: i32[];
    comparisonCount: usize;
    swapCount: usize;
}
```

### Usage example

```tyrus
import { SortInfo, mergeSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = mergeSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 5
print(info.swapCount); // 0
```
