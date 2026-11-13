---
title: kosarajuScc
description: "Kosaraju strongly connected components from tyrus:algorithm."
---

# kosarajuScc

`kosarajuScc(graph)` partitions a directed graph into strongly connected components using a finishing-order traversal followed by traversal of the reversed graph. It accepts an adjacency list or `Graph<N, W>`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `kosarajuScc(graph: usize[][])` | `usize[][]` | O(V + E) |
| `kosarajuScc(graph: Graph<N, W>)` | `usize[][]` | O(V + E) |

Each returned inner array is one maximal mutually reachable set of vertices. Component ordering is not a stability guarantee.

## Example

<!-- @include: ../../.fragments/modules/algorithm/kosaraju-scc/example.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/kosaraju-scc/study-result.md -->
