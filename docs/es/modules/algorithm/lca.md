---
title: lcaBinaryLifting
description: "Tablas de menor ancestro común para árboles de tyrus:algorithm."
---

# lcaBinaryLifting

`lcaBinaryLifting(adj: usize[][], root: usize): Lca` construye tablas de binary lifting sobre el árbol no dirigido descrito por la lista de adyacencia `adj`, donde `adj[v]` guarda los vecinos del vértice `v` y `root` ancla cada consulta. La `Lca` devuelta es un handle de propiedad del llamador de `tyrus:algorithm`; expone `query(u, v)`, que devuelve el menor ancestro común de los vértices `u` y `v` en O(log n) con binary lifting.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `lcaBinaryLifting(adj: usize[][], root: usize)` | Construir tablas de LCA sobre el árbol `adj`, enraizado en `root` |

## Métodos

| Método | Firma | Devuelve | Descripción |
|--------|-------|----------|-------------|
| `query` | `(u: usize, v: usize)` | `usize` | Menor ancestro común de los vértices `u` y `v` |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| construcción | O(n log n), tablas de binary lifting |
| `query` | O(log n) |
| memoria | O(n log n) |

## Ejemplos

### árbol balanceado

<!-- @include: ../../../.fragments/modules/algorithm/lca/balanced-tree.md -->

### camino profundo

<!-- @include: ../../../.fragments/modules/algorithm/lca/deep-path.md -->

### único vértice

<!-- @include: ../../../.fragments/modules/algorithm/lca/single-vertex.md -->

### importación por namespace

<!-- @include: ../../../.fragments/modules/algorithm/lca/namespace-import.md -->
