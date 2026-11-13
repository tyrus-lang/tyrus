---
title: findCutVertices
description: "Articulation vertex detection from tyrus:algorithm."
---

# findCutVertices

`findCutVertices(graph)` returns vertices whose removal disconnects an undirected graph. It accepts an adjacency list or `Graph<N, W>`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `findCutVertices(graph: usize[][])` | `usize[]` | O(V + E) |
| `findCutVertices(graph: Graph<N, W>)` | `usize[]` | O(V + E) |

The adjacency list is interpreted as undirected and should contain symmetric entries. Each result appears once, even when it separates multiple subgraphs.

## Example

<!-- @include: ../../.fragments/modules/algorithm/find-cut-vertices/example.md -->

## Study result

`findCutVerticesInfo` returns `FindCutVerticesInfo`, with copied cut vertices and shared low-link metrics.

<!-- @include: ../../.fragments/modules/algorithm/find-cut-vertices/study-result.md -->
