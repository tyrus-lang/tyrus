---
title: ahoCorasick
description: "Find multiple patterns in one text."
---

# ahoCorasick

Searches all patterns in one text and returns `[endIndex, patternIndex]` pairs. `endIndex` is the inclusive byte offset of the match; pairs are ordered by end offset and then pattern index.

<!-- @include: ../../.fragments/modules/algorithm/aho-corasick/ahocorasick.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/aho-corasick/study-result.md -->

The study result stores one automaton state per input byte. Its additional storage is O(n + p), where `n` is the text length and `p` is the total pattern length.

The automaton runs in O(text length + total pattern length + number of matches) time and uses O(total pattern length) auxiliary space.
