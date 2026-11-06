---
title: insertionSort
description: "Insertion sort en tyrus:algorithm."
---

# insertionSort

`insertionSort(values)` devuelve una copia creciente insertando cada valor en el prefijo ya ordenado. Es útil para arrays pequeños o casi ordenados.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `insertionSort(values: i32[])` | `i32[]` | O(n²) en el peor caso, O(n) en el mejor |

<!-- @include: ../../../.fragments/modules/algorithm/insertion-sort/insertionsort.md -->

## Resultado de estudio

| Exportación | Tipo | Propósito |
|-------------|------|-----------|
| `insertionSortInfo(values: i32[])` | Función | Devuelve un snapshot independiente `SortInfo` y métricas de la operación |
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
import { SortInfo, insertionSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = insertionSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 6
print(info.swapCount); // 5
```
