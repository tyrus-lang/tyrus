---
title: Bubble sort
description: "Bubble Sort directo y un resultado de estudio con métricas deterministas."
---

# Bubble sort

El módulo ofrece dos funciones importadas. `bubbleSort(values)` devuelve una copia ascendente para uso directo, mientras que `bubbleSortInfo(values)` devuelve snapshots independientes de la entrada y la salida y contadores semánticos para estudiar el algoritmo. Ninguna función modifica el array de entrada.

## Ejecución directa

| Firma | Retorno | Complejidad |
|-------|---------|-------------|
| `bubbleSort(values: i32[])` | `i32[]` | O(n²) en el peor caso y en el caso medio, O(n) en el mejor caso |

<!-- @include: ../../../.fragments/modules/algorithm/bubble-sort/direct-execution.md -->

## Resultado de estudio

| Export | Tipo | Finalidad |
|--------|------|-----------|
| `bubbleSortInfo(values: i32[])` | Función | Devuelve un `BubbleSortInfo` sin modificar `values` |
| `BubbleSortInfo` | Interfaz de datos | Contiene los snapshots y todas las métricas de Bubble Sort |

<!-- @include: ../../../.fragments/modules/algorithm/bubble-sort/bubble-sort-info.md -->

`BubbleSortInfo` expone `originalValues: i32[]`, `sortedValues: i32[]`, `comparisonCount: usize`, `swapCount: usize` y `passCount: usize`. Los arrays son snapshots independientes de la entrada y de la salida ordenada.

<!-- @include: ../../../.fragments/modules/algorithm/bubble-sort/study-result.md -->

## Definiciones de las métricas

- `comparisonCount` incrementa una vez por cada par adyacente evaluado por el bucle interno.
- `swapCount` incrementa una vez cuando una comparación adyacente intercambia dos valores.
- `passCount` incrementa después de cada recorrido completado del prefijo aún no ordenado, incluido el recorrido final que no hace intercambios y termina anticipadamente.
- Las entradas vacías o con un elemento realizan cero comparaciones, intercambios y pasadas.

Bubble Sort es estable porque los valores iguales nunca se intercambian. La variante instrumentada tiene el mismo tiempo O(n²) en el peor caso y O(n) en el mejor caso que la función directa, además de O(n) de almacenamiento en el resultado para los snapshots original y ordenado. Las características estáticas como estabilidad y complejidad permanecen en esta documentación en vez de repetirse en cada resultado. La API actual acepta intencionalmente solo `i32[]`.
