---
title: timSort
description: "Ordenação híbrida por comparação em tyrus:algorithm."
---

# timSort

`timSort(values)` retorna uma cópia crescente usando uma ordenação híbrida otimizada.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `timSort(values: i32[])` | `i32[]` | O(n log n) no pior caso |

<!-- @include: ../../../.fragments/modules/algorithm/tim-sort/timsort.md -->

## Resultado de estudo

| Exportação | Tipo | Finalidade |
|------------|------|------------|
| `timSortInfo(values: i32[])` | Função | Retorna um snapshot independente `SortInfo` e métricas da operação |
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
import { SortInfo, timSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = timSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 6
print(info.swapCount); // 0
```
