---
title: Interpolation Search
description: "Busca arrays de enteros uniformemente distribuidos por interpolación."
---

# Interpolation Search

Busca un `i32[]` ascendente estimando la posición del objetivo a partir de los valores en los límites actuales. Devuelve un `usize?` zero-based, o ausencia cuando falta el objetivo. El array debe estar ordenado de forma ascendente.

<!-- @include: ../../../.fragments/modules/algorithm/interpolation-search/interpolation-search.md -->

## Resultado de estudio

`interpolationSearchInfo` devuelve un `InterpolationSearchInfo`, incluida cada posición estimada usada por la búsqueda.

<!-- @include: ../../../.fragments/modules/algorithm/interpolation-search/study-result.md -->

La complejidad media es O(log log n) para valores uniformemente distribuidos, con O(n) en el peor caso y espacio auxiliar O(1).
