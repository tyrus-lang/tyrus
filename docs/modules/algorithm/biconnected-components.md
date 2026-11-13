---
title: biconnectedComponents
description: "Biconnected component decomposition from tyrus:algorithm."
---

# biconnectedComponents

`biconnectedComponents(graph)` decomposes an undirected graph into edge-biconnected components using a low-link traversal. It accepts an adjacency list or `Graph<N, W>`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `biconnectedComponents(graph: usize[][])` | `[usize, usize][][]` | O(V + E) |
| `biconnectedComponents(graph: Graph<N, W>)` | `[usize, usize][][]` | O(V + E) |

Each inner array contains the edges of one component as vertex pairs. Bridges form singleton components, while edges on the same cycle belong together.

## Example

<!-- @include: ../../.fragments/modules/algorithm/biconnected-components/example.md -->

## Study result

`biconnectedComponentsInfo` returns `BiconnectedComponentsInfo`, with copied components and shared low-link metrics.

<!-- @include: ../../.fragments/modules/algorithm/biconnected-components/study-result.md -->
