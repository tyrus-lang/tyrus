---
title: heapSort
description: "In-place heap sort from tyrus:algorithm."
---

# heapSort

`heapSort(values)` returns an ascending copy produced with a binary heap. The native operation uses constant auxiliary algorithmic space apart from the returned array.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `heapSort(values: i32[])` | `i32[]` | O(n log n) time, O(1) auxiliary space |

<!-- @include: ../../.fragments/modules/algorithm/heap-sort/heapsort.md -->

## Study result

| Export | Type | Purpose |
|--------|------|---------|
| `heapSortInfo(values: i32[])` | Function | Returns an independent `SortInfo` snapshot and operation metrics |
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
import { SortInfo, heapSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = heapSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 6
print(info.swapCount); // 5
```
