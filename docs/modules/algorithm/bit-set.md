---
title: BitSet
description: "Compact bit set data structure with set-theoretic operations from tyrus:algorithm."
---

# BitSet

`BitSet` implements a compact set of bits over a fixed capacity, backed by an array of 64-bit words, with set-theoretic operations.

## Construction

| Syntax | Description |
|--------|-------------|
| `BitSet(n)` | Create a bit set with capacity for `n` bits, all initially cleared |
| `BitSet.fromArray(arr)` | Create a bit set with the bits at the positions in `arr` set |

`fromArray` requires a `usize[]` argument. Bit positions are zero-indexed from `0` to `n - 1`.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `set` | `(index: usize, value?: boolean) => void` | `void` | Set, clear, or conditionally set the bit at `index` (default: `true`) |
| `flip` | `(index: usize) => void` | `void` | Toggle the bit at `index` |
| `get` | `(index: usize) => boolean` | `boolean` | Whether the bit at `index` is set |
| `count` | `() => usize` | `usize` | Number of set bits |
| `isEmpty` | `() => boolean` | `boolean` | Whether no bits are set |
| `isSubset` | `(other: BitSet) => boolean` | `boolean` | Whether every set bit is also set in `other` |
| `isSuperset` | `(other: BitSet) => boolean` | `boolean` | Whether every set bit in `other` is also set |
| `union` | `(other: BitSet) => BitSet` | `BitSet` | Bit set with all bits from both sets |
| `intersection` | `(other: BitSet) => BitSet` | `BitSet` | Bit set with bits set in both sets |
| `difference` | `(other: BitSet) => BitSet` | `BitSet` | Bit set with bits set here but not in `other` |
| `complement` | `() => BitSet` | `BitSet` | Bit set with every bit flipped over the capacity |
| `clear` | `() => void` | `void` | Clear all bits |

`union`, `intersection`, `difference`, and `complement` return a fresh `BitSet` owned by the caller; it is released automatically when its binding goes out of scope.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `set` | O(1) |
| `flip` | O(1) |
| `get` | O(1) |
| `count` | O(n), number of machine words |
| `isEmpty` | O(n), number of machine words |
| `isSubset` | O(n), number of machine words |
| `isSuperset` | O(n), number of machine words |
| `union` | O(n), number of machine words |
| `intersection` | O(n), number of machine words |
| `difference` | O(n), number of machine words |
| `complement` | O(n), number of machine words |
| `clear` | O(n), number of machine words |

## Examples

### Construction

<!-- @include: ../../.fragments/modules/algorithm/bit-set/construction.md -->

### set and get

<!-- @include: ../../.fragments/modules/algorithm/bit-set/set-and-get.md -->

### flip

<!-- @include: ../../.fragments/modules/algorithm/bit-set/flip.md -->

### count and isEmpty

<!-- @include: ../../.fragments/modules/algorithm/bit-set/count-and-isempty.md -->

### isSubset and isSuperset

<!-- @include: ../../.fragments/modules/algorithm/bit-set/issubset-and-issuperset.md -->

### union, intersection, and difference

<!-- @include: ../../.fragments/modules/algorithm/bit-set/union-intersection-and-difference.md -->

### complement

<!-- @include: ../../.fragments/modules/algorithm/bit-set/complement.md -->

### fromArray

<!-- @include: ../../.fragments/modules/algorithm/bit-set/fromarray.md -->

### clear

<!-- @include: ../../.fragments/modules/algorithm/bit-set/clear.md -->
