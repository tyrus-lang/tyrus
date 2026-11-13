---
title: mergeSort
description: "Merge sort estable en tyrus:algorithm."
---

# mergeSort

`mergeSort(values)` devuelve una copia creciente estable de `i32[]`.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `mergeSort(values: i32[])` | `i32[]` | O(n log n) de tiempo, O(n) de memoria |

<!-- @include: ../../../.fragments/modules/algorithm/merge-sort/mergesort.md -->

## Resultado de estudio

| Exportación | Tipo | Propósito |
|-------------|------|-----------|
| `mergeSortInfo(values: i32[])` | Función | Devuelve un snapshot independiente `SortInfo` y métricas de la operación |
| `SortInfo` | Interfaz de datos | Contiene los arrays original y ordenado, el recuento de comparaciones y el recuento de intercambios |

`SortInfo` expone `originalValues`, `sortedValues`, `comparisonCount` y `swapCount`. Las ordenaciones sin comparación devuelven cero comparaciones y cero intercambios porque no comparan valores ni intercambian elementos del array. El array de entrada permanece sin cambios.
### Ejemplo de interfaz

```tyrus
interface SortInfo {
    originalValues: i32[];
    sortedValues: i32[];
    comparisonCount: usize;
    swapCount: usize;
}
```

### Ejemplo de uso

```tyrus
import { SortInfo, mergeSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = mergeSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 5
print(info.swapCount); // 0
```
