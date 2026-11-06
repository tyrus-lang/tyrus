---
title: Binary Search
description: "Search an ascending integer array with binary search."
---

# Binary Search

Searches an ascending `i32[]` and returns the zero-based index as `usize?`. The result is absent when the value does not occur. The array must already be sorted in ascending order.

<!-- @include: ../../.fragments/modules/algorithm/binary-search/binary-search.md -->

## Study result

`binarySearchInfo` returns a `SearchInfo` with an independent input snapshot and deterministic comparisons and probes.

<!-- @include: ../../.fragments/modules/algorithm/binary-search/study-result.md -->

The search runs in O(log n) time and uses O(1) auxiliary space.
