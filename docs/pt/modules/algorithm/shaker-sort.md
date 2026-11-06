---
title: shakerSort
description: "Bubble sort bidirecional em tyrus:algorithm."
---

# shakerSort

`shakerSort(values)` retorna uma cópia crescente usando passadas bubble alternadas para frente e para trás.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `shakerSort(values: i32[])` | `i32[]` | O(n²) no pior caso, O(n) no melhor |

<!-- @include: ../../../.fragments/modules/algorithm/shaker-sort/shakersort.md -->

## Resultado de estudo

| Exportação | Tipo | Finalidade |
|------------|------|------------|
| `shakerSortInfo(values: i32[])` | Função | Retorna um snapshot independente `SortInfo` e métricas da operação |
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
import { SortInfo, shakerSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = shakerSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 6
print(info.swapCount); // 5
```
