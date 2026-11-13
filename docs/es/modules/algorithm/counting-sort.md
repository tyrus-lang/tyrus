---
title: countingSort
description: "Counting sort en tyrus:algorithm."
---

# countingSort

`countingSort(values, maxValue)` devuelve una copia creciente de enteros no negativos en el intervalo inclusivo `0..maxValue`.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `countingSort(values: i32[], maxValue: i32)` | `i32[]` | O(n + maxValue) de tiempo y memoria |

<!-- @include: ../../../.fragments/modules/algorithm/counting-sort/countingsort.md -->

## Resultado de estudio

| Exportación | Tipo | Propósito |
|-------------|------|-----------|
| `countingSortInfo(values: i32[], maxValue: i32)` | Función | Devuelve un snapshot independiente `SortInfo` y métricas de la operación |
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
import { SortInfo, countingSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = countingSortInfo(input, 9);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 0
print(info.swapCount); // 0
```
