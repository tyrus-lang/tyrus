---
title: quickSort
description: "Quick sort from tyrus:algorithm."
---

# quickSort

`quickSort(values)` returns an ascending copy using partitioning. It is unstable and has average O(n log n) performance.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `quickSort(values: i32[])` | `i32[]` | O(n log n) average, O(n²) worst case |

<!-- @include: ../../.fragments/modules/algorithm/quick-sort/quicksort.md -->

## Study result

| Export | Type | Purpose |
|--------|------|---------|
| `quickSortInfo(values: i32[])` | Function | Returns an independent `SortInfo` snapshot and operation metrics |
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
import { SortInfo, quickSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = quickSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 8
print(info.swapCount); // 1
```
