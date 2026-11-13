---
title: shellSort
description: "Shell sort from tyrus:algorithm."
---

# shellSort

`shellSort(values)` returns an ascending copy using gap-based insertion passes. It reduces long-distance inversions before the final insertion pass.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `shellSort(values: i32[])` | `i32[]` | O(n²) worst case, O(n log n) typical |

<!-- @include: ../../.fragments/modules/algorithm/shell-sort/shellsort.md -->

## Study result

| Export | Type | Purpose |
|--------|------|---------|
| `shellSortInfo(values: i32[])` | Function | Returns an independent `SortInfo` snapshot and operation metrics |
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
import { SortInfo, shellSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = shellSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 7
print(info.swapCount); // 0
```
