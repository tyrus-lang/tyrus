---
title: bucketSort
description: "Bucket sort em tyrus:algorithm."
---

# bucketSort

`bucketSort(values)` distribui um `i32[]` em buckets proporcionais ao intervalo e ordena cada bucket antes de concatená-los.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `bucketSort(values: i32[])` | `i32[]` | O(n + k) esperado, O(n²) no pior caso |

<!-- @include: ../../../.fragments/modules/algorithm/bucket-sort/bucketsort.md -->

## Resultado de estudo

| Exportação | Tipo | Finalidade |
|------------|------|------------|
| `bucketSortInfo(values: i32[])` | Função | Retorna um snapshot independente `SortInfo` e métricas da operação |
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
import { SortInfo, bucketSortInfo } from "tyrus:algorithm";

const input: i32[] = [8, 3, 5, 1];
const info: SortInfo = bucketSortInfo(input);

print(info.originalValues); // [8, 3, 5, 1]
print(info.sortedValues); // [1, 3, 5, 8]
print(info.comparisonCount); // 1
print(info.swapCount); // 0
```
