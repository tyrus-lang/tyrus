---
title: quickSort
description: "Quick sort em tyrus:algorithm."
---

# quickSort

`quickSort(values)` retorna uma cópia crescente usando particionamento. É instável e tem desempenho médio O(n log n).

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `quickSort(values: i32[])` | `i32[]` | O(n log n) médio, O(n²) no pior caso |

<!-- @include: ../../../.fragments/modules/algorithm/quick-sort/quicksort.md -->

## Resultado de estudo

| Exportação | Tipo | Finalidade |
|------------|------|------------|
| `quickSortInfo(values: i32[])` | Função | Retorna um snapshot independente `SortInfo` e métricas da operação |
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
import { SortInfo, quickSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = quickSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 8
print(info.swapCount); // 1
```
