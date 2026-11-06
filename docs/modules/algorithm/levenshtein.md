---
title: levenshtein
description: "Calculate the byte-based edit distance between two strings."
---

# levenshtein

Returns the minimum number of insertions, deletions, and substitutions needed to transform one string into another. Distance is measured over UTF-8 bytes.

<!-- @include: ../../.fragments/modules/algorithm/levenshtein/levenshtein.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/levenshtein/study-result.md -->

The study result retains the rolling row used by the calculation and reports its dimensions and evaluated cells without returning a quadratic matrix.

The algorithm runs in O(nm) time and O(m) auxiliary space.
