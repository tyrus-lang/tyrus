---
title: Interpolation Search
description: "Search uniformly distributed integer arrays by interpolation."
---

# Interpolation Search

Searches an ascending `i32[]` by estimating the target position from the values at the current bounds. It returns a zero-based `usize?`, or absence when the target is missing. The array must be sorted in ascending order.

<!-- @include: ../../.fragments/modules/algorithm/interpolation-search/interpolation-search.md -->

## Study result

`interpolationSearchInfo` returns an `InterpolationSearchInfo`, including every estimated position used by the search.

<!-- @include: ../../.fragments/modules/algorithm/interpolation-search/study-result.md -->

The average complexity is O(log log n) for uniformly distributed values, with O(n) worst-case complexity and O(1) auxiliary space.
