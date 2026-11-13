---
title: rabinKarp
description: "Find substring matches with a rolling hash."
---

# rabinKarp

Returns every zero-based byte offset where `pattern` occurs in `text`. Hash matches are verified against the original bytes, so collisions do not create false results.

<!-- @include: ../../.fragments/modules/algorithm/rabin-karp/rabinkarp.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/rabin-karp/study-result.md -->

The expected complexity is O(n + m), with O(1) auxiliary space besides the result.
