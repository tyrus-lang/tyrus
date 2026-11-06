---
title: Graph
description: "Estructura de datos de grafo dirigido ponderado de tyrus:algorithm."
---

# Graph

`Graph<N, W>` representa un grafo dirigido ponderado. Los nodos se identifican por valores de tipo `N` y las aristas llevan un peso de tipo `W`. Los nodos se crean automáticamente al añadir una arista, por lo que construir un grafo nunca requiere registro por separado.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `Graph<N, W>()` | Crea un nuevo grafo dirigido vacío con tipo de nodo `N` y tipo de peso `W` |

Los tipos de nodo y peso admitidos son `string`, `bool` y los tipos numéricos `i8`–`i64`, `u8`–`u64`, `f32` y `f64`. Los dos parámetros de tipo deben declararse explícitamente; no se pueden inferir.

## Métodos

| Método | Firma | Devuelve | Descripción |
|--------|-------|----------|-------------|
| `addEdge` | `(source: N, target: N, weight: W) => void` | `void` | Añade una arista; crea los nodos si faltan, sustituye el peso si el par ordenado ya tiene arista |
| `addNode` | `(node: N) => void` | `void` | Añade un nodo aislado |
| `removeNode` | `(node: N) => boolean` | `boolean` | Elimina un nodo y todas sus aristas incidentes; devuelve `true` si el nodo existía |
| `removeEdge` | `(source: N, target: N) => boolean` | `boolean` | Elimina la arista entre `source` y `target`; los nodos permanecen |
| `hasNode` | `(node: N) => boolean` | `boolean` | Si el nodo existe |
| `hasEdge` | `(source: N, target: N) => boolean` | `boolean` | Si el par ordenado tiene una arista |
| `edge` | `(source: N, target: N) => W?` | `W?` | El peso de la arista, o vacío cuando no hay arista |
| `neighbors` | `(node: N) => N[]` | `N[]` | Los vecinos de salida de un nodo, en orden de inserción |
| `degree` | `(node: N) => number` | `number` | El grado de salida de un nodo |
| `nodeCount` | `() => number` | `number` | El número total de nodos |
| `edgeCount` | `() => number` | `number` | El número total de aristas |
| `isEmpty` | `() => boolean` | `boolean` | Si el grafo no tiene nodos |
| `nodes` | `() => N[]` | `N[]` | Todos los nodos, en orden de inserción |
| `edges` | `() => [N, N, W][]` | `[N, N, W][]` | Todas las aristas como tuplas `(source, target, weight)` |
| `reverse` | `() => Graph<N, W>` | `Graph<N, W>` | Un nuevo grafo con la dirección de todas las aristas invertida |
| `mapEdges` | `((weight: W) => W') => Graph<N, W'>` | `Graph<N, W'>` | Un nuevo grafo con pesos transformados por el callback |
| `mapNodes` | `((node: N) => N') => Graph<N', W>` | `Graph<N', W>` | Un nuevo grafo con valores de nodo transformados por el callback |
| `toDot` | `() => string` | `string` | El grafo renderizado como DOT de Graphviz |
| `toString` | `() => string` | `string` | Una representación compacta de adyacencia en una línea |

`neighbors`, `nodes` y `edges` devuelven arrays recién asignados que pertenecen al llamador. El handle nativo del grafo se libera automáticamente cuando la variable sale de ámbito.

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `addEdge` / `addNode` | O(1) amortizado, O(1) esperado |
| `hasNode` / `edge` / `hasEdge` | O(1) esperado |
| `removeNode` | O(V + E) |
| `removeEdge` | O(grade(source)) |
| `neighbors` / `degree` | O(grade(node)) |
| `nodes` / `nodeCount` / `isEmpty` | O(V) / O(1) |
| `edges` / `edgeCount` | O(E) / O(1) |
| `reverse` / `mapEdges` / `mapNodes` | O(V + E) |
| `toDot` / `toString` | O(V + E) |

## Ejemplos

### Construir un grafo

<!-- @include: ../../../.fragments/modules/algorithm/graph/building-a-graph.md -->

### Consultas

<!-- @include: ../../../.fragments/modules/algorithm/graph/queries.md -->

### Iteración

<!-- @include: ../../../.fragments/modules/algorithm/graph/iteration.md -->

### Mutación

<!-- @include: ../../../.fragments/modules/algorithm/graph/mutation.md -->

### Transformaciones

<!-- @include: ../../../.fragments/modules/algorithm/graph/transformations.md -->

### Renderización

<!-- @include: ../../../.fragments/modules/algorithm/graph/rendering.md -->

### Nodos numéricos y pesos float

<!-- @include: ../../../.fragments/modules/algorithm/graph/numeric-nodes-and-float-weights.md -->
