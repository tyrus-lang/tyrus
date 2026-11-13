---
title: radixSort
description: "Radix sort from tyrus:algorithm."
---

# radixSort

`radixSort(values)` returns an ascending copy of signed `i32` values using four stable least-significant-byte passes.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `radixSort(values: i32[])` | `i32[]` | O(4n) time, O(n) memory |

<!-- @include: ../../.fragments/modules/algorithm/radix-sort/radixsort.md -->

## Study result

| Export | Type | Purpose |
|--------|------|---------|
| `radixSortInfo(values: i32[])` | Function | Returns an independent `SortInfo` snapshot and operation metrics |
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
import { SortInfo, radixSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = radixSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 0
print(info.swapCount); // 0
```
