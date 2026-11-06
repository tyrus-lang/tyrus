---
title: countingSort
description: "Counting sort from tyrus:algorithm."
---

# countingSort

`countingSort(values, maxValue)` returns an ascending copy of non-negative integers in the inclusive range `0..maxValue`. Values outside that range are returned unchanged.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `countingSort(values: i32[], maxValue: i32)` | `i32[]` | O(n + maxValue) time and memory |

<!-- @include: ../../.fragments/modules/algorithm/counting-sort/countingsort.md -->

## Study result

| Export | Type | Purpose |
|--------|------|---------|
| `countingSortInfo(values: i32[], maxValue: i32)` | Function | Returns an independent `SortInfo` snapshot and operation metrics |
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
import { SortInfo, countingSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = countingSortInfo(input, 9);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 0
print(info.swapCount); // 0
```
