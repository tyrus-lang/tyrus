---
title: countingSort
description: "Counting sort em tyrus:algorithm."
---

# countingSort

`countingSort(values, maxValue)` retorna uma cópia crescente de inteiros não negativos no intervalo inclusivo `0..maxValue`.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `countingSort(values: i32[], maxValue: i32)` | `i32[]` | O(n + maxValue) de tempo e memória |

<!-- @include: ../../../.fragments/modules/algorithm/counting-sort/countingsort.md -->

## Resultado de estudo

| Exportação | Tipo | Finalidade |
|------------|------|------------|
| `countingSortInfo(values: i32[], maxValue: i32)` | Função | Retorna um snapshot independente `SortInfo` e métricas da operação |
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
import { SortInfo, countingSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = countingSortInfo(input, 9);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 0
print(info.swapCount); // 0
```
