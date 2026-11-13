---
title: selectionSort
description: "Selection sort em tyrus:algorithm."
---

# selectionSort

`selectionSort(values)` retorna uma cópia crescente escolhendo repetidamente o menor valor restante.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `selectionSort(values: i32[])` | `i32[]` | O(n²), O(1) de memória auxiliar |

<!-- @include: ../../../.fragments/modules/algorithm/selection-sort/selectionsort.md -->

## Resultado de estudo

| Exportação | Tipo | Finalidade |
|------------|------|------------|
| `selectionSortInfo(values: i32[])` | Função | Retorna um snapshot independente `SortInfo` e métricas da operação |
| `SortInfo` | Interface de dados | Mantém os arrays original e ordenado, a contagem de comparações e a contagem de trocas |

`SortInfo` expõe `originalValues`, `sortedValues`, `comparisonCount` e `swapCount`. Ordenações sem comparação retornam zero comparações e zero trocas, pois não comparam valores nem trocam elementos do array. O array de entrada permanece inalterado.
### Exemplo da interface

```tyrus
interface SortInfo {
    originalValues: i32[];
    sortedValues: i32[];
    comparisonCount: usize;
    swapCount: usize;
}
```

### Exemplo de uso

```tyrus
import { SortInfo, selectionSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = selectionSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 6
print(info.swapCount); // 1
```
