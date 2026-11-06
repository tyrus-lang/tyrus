---
title: Bubble sort
description: "Direct Bubble Sort and a study result with deterministic metrics."
---

# Bubble sort

The module offers two imported functions. `bubbleSort(values)` returns an ascending copy for direct use, while `bubbleSortInfo(values)` returns independent input/output snapshots and semantic counters for studying the algorithm. Neither function changes the input array.

## Direct execution

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `bubbleSort(values: i32[])` | `i32[]` | O(n²) worst and average case, O(n) best case |

<!-- @include: ../../.fragments/modules/algorithm/bubble-sort/direct-execution.md -->

## Study result

| Export | Kind | Purpose |
|--------|------|---------|
| `bubbleSortInfo(values: i32[])` | Function | Returns a `BubbleSortInfo` without changing `values` |
| `BubbleSortInfo` | Data interface | Holds the Bubble Sort snapshots and all metrics |

<!-- @include: ../../.fragments/modules/algorithm/bubble-sort/bubble-sort-info.md -->

`BubbleSortInfo` exposes `originalValues: i32[]`, `sortedValues: i32[]`, `comparisonCount: usize`, `swapCount: usize`, and `passCount: usize`. The arrays are independent snapshots of the input and sorted output.

<!-- @include: ../../.fragments/modules/algorithm/bubble-sort/study-result.md -->

## Metric definitions

- `comparisonCount` increments once for every adjacent pair evaluated by the inner loop.
- `swapCount` increments once when an adjacent comparison exchanges two values.
- `passCount` increments after each completed traversal of the current unsorted prefix, including the final traversal that performs no swap and stops early.
- Empty and single-element inputs perform zero comparisons, swaps, and passes.

Bubble Sort is stable because equal values are never exchanged. The instrumented variant has the same O(n²) worst-case and O(n) best-case running time as the direct function, plus O(n) result storage for the original and sorted snapshots. Static characteristics such as stability and complexity remain in this documentation instead of being repeated in every result value. The current API intentionally accepts only `i32[]`.
