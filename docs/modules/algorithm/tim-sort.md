---
title: timSort
description: "Hybrid comparison sort from tyrus:algorithm."
---

# timSort

`timSort(values)` returns an ascending copy using the standard library's optimized hybrid comparison sort, combining run detection and insertion/merge techniques.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `timSort(values: i32[])` | `i32[]` | O(n log n) worst case |

<!-- @include: ../../.fragments/modules/algorithm/tim-sort/timsort.md -->

## Study result

| Export | Type | Purpose |
|--------|------|---------|
| `timSortInfo(values: i32[])` | Function | Returns an independent `SortInfo` snapshot and operation metrics |
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
import { SortInfo, timSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = timSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 6
print(info.swapCount); // 0
```
