---
title: minCut
description: "Corte mínimo en tyrus:algorithm."
---

# minCut

`minCut(capacity, source, sink)` calcula un flujo máximo y devuelve su valor y las aristas de capacidad que cruzan el corte mínimo correspondiente.

| Firma | Retorno | Complejidad |
|-------|---------|-------------|
| `minCut(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize][]]` | O(V²E) |

<!-- @include: ../../../.fragments/modules/algorithm/min-cut/mincut.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/min-cut/study-result.md -->
