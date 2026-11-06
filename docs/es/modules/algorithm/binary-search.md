---
title: Binary Search
description: "Busca un array ascendente de enteros con búsqueda binaria."
---

# Binary Search

Busca un `i32[]` ascendente y devuelve el índice zero-based como `usize?`. El resultado está ausente cuando el valor no aparece. El array debe estar ordenado de forma ascendente.

<!-- @include: ../../../.fragments/modules/algorithm/binary-search/binary-search.md -->

## Resultado de estudio

`binarySearchInfo` devuelve un `SearchInfo` con un snapshot independiente de la entrada y comparaciones y probes deterministas.

<!-- @include: ../../../.fragments/modules/algorithm/binary-search/study-result.md -->

La búsqueda se ejecuta en O(log n) y usa espacio auxiliar O(1).
