---
title: Fleury Eulerian
description: "Fleury Eulerian traversal from tyrus:algorithm."
---

# Fleury Eulerian

`fleuryEulerian(graph)` returns a path that uses every edge exactly once, choosing non-bridges while alternatives exist. It accepts an undirected adjacency list or `Graph<N, W>` and returns absence when no Eulerian path exists.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `fleuryEulerian(graph: usize[][])` | `usize[]?` | O(E(V + E)) |
| `fleuryEulerian(graph: Graph<N, W>)` | `usize[]?` | O(E(V + E)) |

The adjacency list represents an undirected graph with symmetric entries. A successful path has `E + 1` vertices.

## Example

<!-- @include: ../../.fragments/modules/algorithm/fleury-eulerian/example.md -->

## Study result

`fleuryEulerianInfo` returns the path together with edge examination and bridge-choice metrics. Import both the base and specialized interfaces explicitly.

<!-- @include: ../../.fragments/modules/algorithm/fleury-eulerian/study-result.md -->
