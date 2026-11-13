---
title: Quickselect
description: "Selecciona el k-ésimo menor valor en un array de enteros."
---

# Quickselect

Selecciona el valor en la posición zero-based `k` del orden ascendente sin ordenar todo el array. La entrada es un `i32[]`; el resultado es `i32?` y está ausente cuando `k` queda fuera del array.

<!-- @include: ../../../.fragments/modules/algorithm/quickselect/quickselect.md -->

## Resultado de estudio

`quickselectInfo` devuelve un `SelectionInfo` con el snapshot de entrada original y métricas deterministas de particiones, comparaciones e intercambios.

<!-- @include: ../../../.fragments/modules/algorithm/quickselect/study-result.md -->

La complejidad media es O(n), con O(n²) en el peor caso y espacio auxiliar O(1).
