---
title: bucketSort
description: "Bucket sort from tyrus:algorithm."
---

# bucketSort

`bucketSort(values)` distributes an `i32[]` across range-proportional buckets and sorts each bucket before concatenation.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `bucketSort(values: i32[])` | `i32[]` | O(n + k) expected, O(n²) worst case |

<!-- @include: ../../.fragments/modules/algorithm/bucket-sort/bucketsort.md -->

## Study result

| Export | Type | Purpose |
|--------|------|---------|
| `bucketSortInfo(values: i32[])` | Function | Returns an independent `SortInfo` snapshot and operation metrics |
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
import { SortInfo, bucketSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = bucketSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 1
print(info.swapCount); // 0
```
