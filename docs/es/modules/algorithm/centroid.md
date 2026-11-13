---
title: centroidDecomposition
description: "Descomposición centroid de árboles de tyrus:algorithm."
---

# centroidDecomposition

`centroidDecomposition(adj: usize[][]): Centroid` construye la descomposición centroid sobre el árbol no dirigido descrito por la lista de adyacencia `adj`, donde `adj[v]` guarda los vecinos del vértice `v`. La `Centroid` devuelta es un handle de propiedad del llamador de `tyrus:algorithm`; todo vértice del árbol de entrada se convierte en un nodo del árbol centroid, y eliminar un centroid divide cada componente en componentes hijos con profundidad de a lo sumo la mitad de su tamaño. Expone `parentOf(v)`, que devuelve el padre en el árbol centroid de `v` como `usize?` (la raíz centroid global no tiene padre), y `depth(v)`, la profundidad de `v` en el árbol centroid (raíz con profundidad 0).

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `centroidDecomposition(adj: usize[][])` | Descomponer el árbol `adj` en un árbol centroid |

## Métodos

| Método | Firma | Devuelve | Descripción |
|--------|-------|----------|-------------|
| `parentOf` | `(v: usize)` | `usize?` | Padre en el árbol centroid de `v`; ausente para la raíz centroid global |
| `depth` | `(v: usize)` | `usize` | Profundidad de `v` en el árbol centroid (raíz con profundidad 0) |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| construcción | O(n), cálculo iterativo de tamaños por componente y búsqueda del centroid |
| `parentOf`, `depth` | O(1) |
| memoria | O(n) |

## Ejemplos

### árbol balanceado

<!-- @include: ../../../.fragments/modules/algorithm/centroid/balanced-tree.md -->

### camino profundo

<!-- @include: ../../../.fragments/modules/algorithm/centroid/deep-path.md -->

### único vértice

<!-- @include: ../../../.fragments/modules/algorithm/centroid/single-vertex.md -->

### importación por namespace

<!-- @include: ../../../.fragments/modules/algorithm/centroid/namespace-import.md -->
