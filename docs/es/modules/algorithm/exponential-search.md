---
title: Exponential Search
description: "Busca un array ascendente de enteros expandiendo el intervalo exponencialmente."
---

# Exponential Search

Encuentra un valor en un `i32[]` ascendente expandiendo el intervalo exponencialmente y aplicando después búsqueda binaria. Devuelve el índice zero-based como `usize?`, o ausencia cuando falta el objetivo.

<!-- @include: ../../../.fragments/modules/algorithm/exponential-search/exponential-search.md -->

## Resultado de estudio

`exponentialSearchInfo` devuelve un `ExponentialSearchInfo` con las expansiones correctas y el intervalo semiabierto final buscado por la búsqueda binaria.

<!-- @include: ../../../.fragments/modules/algorithm/exponential-search/study-result.md -->

La búsqueda se ejecuta en O(log n) y usa espacio auxiliar O(1).
