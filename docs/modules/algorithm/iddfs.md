---
title: iddfs
description: "Iterative-deepening depth-first graph search from tyrus:algorithm."
---

# iddfs

`iddfs(adj, start, maxDepth)` repeats depth-limited DFS and returns the first-reach order discovered up to `maxDepth`. It accepts an adjacency list and returns vertex indices.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `iddfs(adj: usize[][], start: usize, maxDepth: usize)` | `usize[]` | O(V + E) per depth pass; O(V) memory |

<!-- @include: ../../.fragments/modules/algorithm/iddfs/iddfs.md -->

## Study result

`iddfsInfo` returns `IddfsInfo` with the final visit order, attempted depth limits, and deterministic traversal metrics.

<!-- @include: ../../.fragments/modules/algorithm/iddfs/study-result.md -->
