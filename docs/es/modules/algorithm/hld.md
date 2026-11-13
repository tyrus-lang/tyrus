---
title: heavyLightDecomposition
description: "Descomposición heavy-light de árboles de tyrus:algorithm."
---

# heavyLightDecomposition

`heavyLightDecomposition(adj: usize[][], root: usize): Hld` construye una descomposición heavy-light sobre el árbol no dirigido descrito por la lista de adyacencia `adj`, donde `adj[v]` guarda los vecinos del vértice `v` y `root` ancla cada consulta. La `Hld` devuelta es un handle de propiedad del llamador de `tyrus:algorithm`; expone `queryPath(u, v)`, que devuelve los rangos contiguos `[l, r]` (en el orden de `position`) cuya unión cubre la ruta `u → v` del árbol en O(log n), listos para alimentar un segment tree para consultas de rutas.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `heavyLightDecomposition(adj: usize[][], root: usize)` | Descomponer el árbol `adj` en cadenas pesadas, enraizado en `root` |

## Métodos

| Método | Firma | Devuelve | Descripción |
|--------|-------|----------|-------------|
| `queryPath` | `(u: usize, v: usize)` | `[usize, usize][]` | Rangos de cadena cuya unión cubre la ruta `u → v` |
| `position` | `(v: usize)` | `usize` | Posición lineal por cadena del vértice `v` |
| `head` | `(v: usize)` | `usize` | Vértice en la cima de la cadena pesada que contiene a `v` |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| construcción | O(n), un DFS más la asignación de cadenas |
| `queryPath` | O(log n), a lo sumo O(log n) rangos de cadena |
| `position`, `head` | O(1) |
| memoria | O(n) |

## Ejemplos

### árbol balanceado

<!-- @include: ../../../.fragments/modules/algorithm/hld/balanced-tree.md -->

### camino profundo

<!-- @include: ../../../.fragments/modules/algorithm/hld/deep-path.md -->

### único vértice

<!-- @include: ../../../.fragments/modules/algorithm/hld/single-vertex.md -->

### importación por namespace

<!-- @include: ../../../.fragments/modules/algorithm/hld/namespace-import.md -->
