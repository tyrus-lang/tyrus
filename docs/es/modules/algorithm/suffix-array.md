---
title: suffixArray
description: "Construcción de suffix array para strings de tyrus:algorithm."
---

# suffixArray

`suffixArray(text: string): usize[]` devuelve el índice inicial de cada sufijo de `text`, ordenados lexicográficamente comparando los bytes del sufijo. El resultado es un array nuevo de propiedad del llamador, con una entrada por carácter de la entrada; su primer elemento es el índice inicial del sufijo más pequeño. Es una función libre de `tyrus:algorithm`, importada como cualquier otro nombre de ese módulo.

## Función

| Firma | Devuelve | Descripción |
|-------|----------|-------------|
| `suffixArray(text: string)` | `usize[]` | Índice inicial de cada sufijo, ordenados lexicográficamente |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| construcción | O(n² log n), ordenación directa de sufijos |
| memoria | O(n) |

## Ejemplos

### banana

<!-- @include: ../../../.fragments/modules/algorithm/suffix-array/banana.md -->

### vacío y un único carácter

<!-- @include: ../../../.fragments/modules/algorithm/suffix-array/empty-and-single-character.md -->

### importación por namespace

<!-- @include: ../../../.fragments/modules/algorithm/suffix-array/namespace-import.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/suffix-array/study-result.md -->

`sortRoundCount` cuenta las rondas del merge bottom-up, `comparisonCount` cuenta las comparaciones entre sufijos y `comparedByteCount` cuenta los pares de bytes inspeccionados por esas comparaciones. La instrumentación mantiene almacenamiento adicional O(n) y el mismo límite temporal O(n² log n) en el peor caso de la ordenación directa de sufijos.
