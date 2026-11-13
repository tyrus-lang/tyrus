---
title: Counter
description: "Estructura de contador de frecuencias de tyrus:algorithm."
---

# Counter

`Counter<T>` cuenta valores escalares y conserva el orden de la primera inserción cuando las frecuencias empatan.

## Construcción

`Counter<T>()` crea un contador vacío. `T` debe ser un tipo escalar: `boolean`, `char`, entero o punto flotante. Los valores con propiedad en el heap, como `string`, no son compatibles.

## Métodos

| Método | Firma | Devuelve | Descripción |
|--------|-------|----------|-------------|
| `add` | `(T, i32?) => void` | `void` | Incrementa un elemento por una cantidad positiva; el valor predeterminado es 1. |
| `remove` | `(T, i32?) => void` | `void` | Decrementa un elemento por una cantidad positiva; los elementos ausentes o agotados se eliminan. |
| `count` | `(T) => i32` | `i32` | Devuelve la cantidad, o 0 cuando está ausente. |
| `mostCommon` | `(usize) => [T, i32][]` | `[T, i32][]` | Devuelve hasta esa cantidad de pares `[item, count]` en orden de cantidad descendente y estable en empates. |
| `total` | `() => i32` | `i32` | Devuelve la suma de todas las cantidades. |
| `unique` | `() => usize` | `usize` | Devuelve el número de elementos presentes. |
| `isEmpty` | `() => boolean` | `boolean` | Devuelve si el contador está vacío. |
| `clear` | `() => void` | `void` | Elimina todos los elementos. |

`add`, `remove` y `total` terminan con un error de runtime si una cantidad positiva `i32` no puede representarse.

## Complejidad

| Operación | Complejidad |
|-----------|-------------|
| `add`, `remove`, `count` | O(1) promedio |
| `mostCommon(k)` | O(n log n) |
| `total`, `clear` | O(n) |
| `unique`, `isEmpty` | O(1) |

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/counter/example.md -->
