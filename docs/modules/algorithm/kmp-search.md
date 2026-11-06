---
title: kmpSearch
description: "Find all exact substring matches with prefix preprocessing."
---

# kmpSearch

Returns every zero-based byte offset where `pattern` occurs in `text`. An empty pattern matches at every boundary, so the result contains offsets from `0` through `text.length`.

<!-- @include: ../../.fragments/modules/algorithm/kmp-search/kmpsearch.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/kmp-search/study-result.md -->

The algorithm runs in O(n + m) time and uses O(m) auxiliary space.
