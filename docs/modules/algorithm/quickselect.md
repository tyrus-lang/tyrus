---
title: Quickselect
description: "Select the k-th smallest value in an integer array."
---

# Quickselect

Selects the value at zero-based position `k` in the sorted order without sorting the complete array. The input is an `i32[]`; the result is `i32?` and is absent when `k` is outside the array.

<!-- @include: ../../.fragments/modules/algorithm/quickselect/quickselect.md -->

## Study result

`quickselectInfo` returns a `SelectionInfo` with the original input snapshot and deterministic partition, comparison, and swap metrics.

<!-- @include: ../../.fragments/modules/algorithm/quickselect/study-result.md -->

The average complexity is O(n), with O(n²) worst-case complexity and O(1) auxiliary space.
