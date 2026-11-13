---
title: quickSort
description: "Quick sort en tyrus:algorithm."
---

# quickSort

`quickSort(values)` devuelve una copia creciente mediante particionamiento. Es inestable y tiene rendimiento medio O(n log n).

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `quickSort(values: i32[])` | `i32[]` | O(n log n) medio, O(n²) en el peor caso |

<!-- @include: ../../../.fragments/modules/algorithm/quick-sort/quicksort.md -->

## Resultado de estudio

| Exportación | Tipo | Propósito |
|-------------|------|-----------|
| `quickSortInfo(values: i32[])` | Función | Devuelve un snapshot independiente `SortInfo` y métricas de la operación |
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
import { SortInfo, quickSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = quickSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 8
print(info.swapCount); // 1
```
