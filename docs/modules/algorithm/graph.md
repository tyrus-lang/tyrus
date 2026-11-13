---
title: Graph
description: "Directed weighted graph data structure from tyrus:algorithm."
---

# Graph

`Graph<N, W>` represents a directed weighted graph. Nodes are identified by values of type `N` and edges carry a weight of type `W`. Nodes are created automatically when an edge is added, so building a graph never requires separate registration.

## Construction

| Syntax | Description |
|--------|-------------|
| `Graph<N, W>()` | Create a new empty directed graph with node type `N` and weight type `W` |

Supported node and weight types are `string`, `bool`, and the numeric types `i8`–`i64`, `u8`–`u64`, `f32`, and `f64`. The two type parameters must be declared explicitly; they cannot be inferred.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `addEdge` | `(source: N, target: N, weight: W) => void` | `void` | Add an edge; creates the nodes if absent, replaces the weight if the ordered pair already has an edge |
| `addNode` | `(node: N) => void` | `void` | Add an isolated node |
| `removeNode` | `(node: N) => boolean` | `boolean` | Remove a node and all its incident edges; returns `true` if the node existed |
| `removeEdge` | `(source: N, target: N) => boolean` | `boolean` | Remove the edge between `source` and `target`; the nodes remain |
| `hasNode` | `(node: N) => boolean` | `boolean` | Whether the node exists |
| `hasEdge` | `(source: N, target: N) => boolean` | `boolean` | Whether the ordered pair has an edge |
| `edge` | `(source: N, target: N) => W?` | `W?` | The weight of the edge, or empty when there is no edge |
| `neighbors` | `(node: N) => N[]` | `N[]` | The out-neighbors of a node, in insertion order |
| `degree` | `(node: N) => number` | `number` | The out-degree of a node |
| `nodeCount` | `() => number` | `number` | The total number of nodes |
| `edgeCount` | `() => number` | `number` | The total number of edges |
| `isEmpty` | `() => boolean` | `boolean` | Whether the graph has no nodes |
| `nodes` | `() => N[]` | `N[]` | All nodes, in insertion order |
| `edges` | `() => [N, N, W][]` | `[N, N, W][]` | All edges as `(source, target, weight)` tuples |
| `reverse` | `() => Graph<N, W>` | `Graph<N, W>` | A new graph with every edge direction reversed |
| `mapEdges` | `((weight: W) => W') => Graph<N, W'>` | `Graph<N, W'>` | A new graph with weights transformed by the callback |
| `mapNodes` | `((node: N) => N') => Graph<N', W>` | `Graph<N', W>` | A new graph with node values transformed by the callback |
| `toDot` | `() => string` | `string` | The graph rendered as Graphviz DOT |
| `toString` | `() => string` | `string` | A compact one-line adjacency representation |

`neighbors`, `nodes`, and `edges` return freshly allocated arrays that the caller owns. The native graph handle is released automatically when the binding goes out of scope.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `addEdge` / `addNode` | O(1) amortized, O(1) expected |
| `hasNode` / `edge` / `hasEdge` | O(1) expected |
| `removeNode` | O(V + E) |
| `removeEdge` | O(deg(source)) |
| `neighbors` / `degree` | O(deg(node)) |
| `nodes` / `nodeCount` / `isEmpty` | O(V) / O(1) |
| `edges` / `edgeCount` | O(E) / O(1) |
| `reverse` / `mapEdges` / `mapNodes` | O(V + E) |
| `toDot` / `toString` | O(V + E) |

## Examples

### Building a graph

<!-- @include: ../../.fragments/modules/algorithm/graph/building-a-graph.md -->

### Queries

<!-- @include: ../../.fragments/modules/algorithm/graph/queries.md -->

### Iteration

<!-- @include: ../../.fragments/modules/algorithm/graph/iteration.md -->

### Mutation

<!-- @include: ../../.fragments/modules/algorithm/graph/mutation.md -->

### Transformations

<!-- @include: ../../.fragments/modules/algorithm/graph/transformations.md -->

### Rendering

<!-- @include: ../../.fragments/modules/algorithm/graph/rendering.md -->

### Numeric nodes and float weights

<!-- @include: ../../.fragments/modules/algorithm/graph/numeric-nodes-and-float-weights.md -->
