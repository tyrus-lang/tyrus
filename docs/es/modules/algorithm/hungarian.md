---
title: hungarian
description: "Asignación de coste mínimo en tyrus:algorithm."
---

# hungarian

`hungarian(costMatrix)` asigna filas a columnas distintas con coste total mínimo. La entrada es una matriz `i32[][]` y el resultado contiene el coste óptimo y los pares fila-columna.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `hungarian(costMatrix: i32[][])` | `[i32, [usize, usize][]]` | O(n³) para una matriz n × n |

En matrices rectangulares se asigna cada fila cuando hay suficientes columnas. Se aceptan costes positivos y negativos.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/hungarian/example.md -->

## Resultado para estudio

<!-- @include: ../../../.fragments/modules/algorithm/hungarian/study-result.md -->
