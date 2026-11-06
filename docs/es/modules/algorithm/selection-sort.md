---
title: selectionSort
description: "Selection sort en tyrus:algorithm."
---

# selectionSort

`selectionSort(values)` devuelve una copia creciente eligiendo repetidamente el menor valor restante.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `selectionSort(values: i32[])` | `i32[]` | O(n²), O(1) de memoria auxiliar |

<!-- @include: ../../../.fragments/modules/algorithm/selection-sort/selectionsort.md -->

## Resultado de estudio

| Exportación | Tipo | Propósito |
|-------------|------|-----------|
| `selectionSortInfo(values: i32[])` | Función | Devuelve un snapshot independiente `SortInfo` y métricas de la operación |
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
import { SortInfo, selectionSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = selectionSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 6
print(info.swapCount); // 1
```
