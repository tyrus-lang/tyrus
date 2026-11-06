---
title: Trie
description: "Prefix tree data structure for string words from tyrus:algorithm."
---

# Trie

`Trie` implements a prefix tree for string words with O(m) search, prefix, insertion, and lookup, where m is the length of the word. It is non-generic: `Trie()` creates an empty trie storing arbitrary-length strings.

## Construction

| Syntax | Description |
|--------|-------------|
| `Trie()` | Create a new empty trie |

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `insert` | `(word: string) => void` | `void` | Insert a copy of `word`; duplicate insertions are ignored |
| `search` | `(word: string) => boolean` | `boolean` | Whether `word` is stored as a complete word |
| `startsWith` | `(prefix: string) => boolean` | `boolean` | Whether any stored word has the given prefix |
| `len` | `() => usize` | `usize` | Number of distinct stored words |
| `isEmpty` | `() => boolean` | `boolean` | Whether the trie stores no words |
| `clear` | `() => void` | `void` | Remove all words |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `insert` | O(m), word length |
| `search` | O(m), word length |
| `startsWith` | O(m), prefix length |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n), number of stored nodes |

## Examples

### insert

<!-- @include: ../../.fragments/modules/algorithm/trie/insert.md -->

### search

<!-- @include: ../../.fragments/modules/algorithm/trie/search.md -->

### startsWith

<!-- @include: ../../.fragments/modules/algorithm/trie/startswith.md -->

### len and isEmpty

<!-- @include: ../../.fragments/modules/algorithm/trie/len-and-isempty.md -->

### clear

<!-- @include: ../../.fragments/modules/algorithm/trie/clear.md -->
