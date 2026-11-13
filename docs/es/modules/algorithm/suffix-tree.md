---
title: suffixTreeUkkonen
description: "Construcción de suffix tree para strings de tyrus:algorithm."
---

# suffixTreeUkkonen

`suffixTreeUkkonen(text: string): SuffixTree` construye un suffix tree sobre `text` con el algoritmo lineal de Ukkonen. La `SuffixTree` devuelta es un handle de propiedad del llamador de `tyrus:algorithm`; expone `getSuffixArray()`, que devuelve el índice inicial de cada sufijo de `text`, ordenados lexicográficamente comparando los bytes del sufijo, como un array nuevo de propiedad del llamador con una entrada por carácter de la entrada.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `suffixTreeUkkonen(text: string)` | Construir un suffix tree sobre `text` |

## Métodos

| Método | Firma | Devuelve | Descripción |
|--------|-------|----------|-------------|
| `getSuffixArray` | `() => usize[]` | `usize[]` | Índice inicial de cada sufijo, ordenados lexicográficamente |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| construcción | O(n), algoritmo de Ukkonen |
| `getSuffixArray` | O(n), un único recorrido del árbol |
| memoria | O(n) |

## Ejemplos

### banana

<!-- @include: ../../../.fragments/modules/algorithm/suffix-tree/banana.md -->

### tipo anotado

<!-- @include: ../../../.fragments/modules/algorithm/suffix-tree/annotated-type.md -->

### vacío y un único carácter

<!-- @include: ../../../.fragments/modules/algorithm/suffix-tree/empty-and-single-character.md -->

### importación por namespace

<!-- @include: ../../../.fragments/modules/algorithm/suffix-tree/namespace-import.md -->
