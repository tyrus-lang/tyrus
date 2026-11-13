---
title: boyerMoore
description: "Find substring matches with the bad-character rule."
---

# boyerMoore

Returns every zero-based byte offset where `pattern` occurs in `text`, skipping comparisons with a bad-character shift.

<!-- @include: ../../.fragments/modules/algorithm/boyer-moore/boyermoore.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/boyer-moore/study-result.md -->

The study result runs the same bad-character search while recording each comparison and applied shift. Its extra storage is O(m + s), where `m` is the pattern length and `s` is the number of inspected alignments.

The search uses O(nm) worst-case time and O(1) auxiliary space for its byte lookup table.
