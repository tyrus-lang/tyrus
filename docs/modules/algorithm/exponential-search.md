---
title: Exponential Search
description: "Search an ascending integer array with exponential range expansion."
---

# Exponential Search

Finds a value in an ascending `i32[]` by expanding the search range exponentially and then applying binary search. It returns the zero-based index as `usize?`, or absence when the target is missing.

<!-- @include: ../../.fragments/modules/algorithm/exponential-search/exponential-search.md -->

## Study result

`exponentialSearchInfo` returns an `ExponentialSearchInfo` with the successful range expansions and the final half-open range searched by binary search.

<!-- @include: ../../.fragments/modules/algorithm/exponential-search/study-result.md -->

The search runs in O(log n) time and uses O(1) auxiliary space.
