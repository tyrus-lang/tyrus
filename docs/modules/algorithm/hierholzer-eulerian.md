---
title: Hierholzer Eulerian
description: "Hierholzer Eulerian traversal from tyrus:algorithm."
---

# Hierholzer Eulerian

`hierholzerEulerian(graph)` constructs a path that uses every edge exactly once by splicing closed walks. It accepts an undirected adjacency list or `Graph<N, W>` and returns absence when no Eulerian path exists.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `hierholzerEulerian(graph: usize[][])` | `usize[]?` | O(V + E) |
| `hierholzerEulerian(graph: Graph<N, W>)` | `usize[]?` | O(V + E) |

The adjacency list represents an undirected graph with symmetric entries. A successful path has `E + 1` vertices.

## Example

<!-- @include: ../../.fragments/modules/algorithm/hierholzer-eulerian/example.md -->

## Study result

`hierholzerEulerianInfo` returns the path together with edge and stack-operation metrics. Import both the base and specialized interfaces explicitly.

<!-- @include: ../../.fragments/modules/algorithm/hierholzer-eulerian/study-result.md -->
