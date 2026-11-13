---
title: Constantes
description: Constantes matemáticas exportadas por tyrus:math.
---

# Constantes

Todas las constantes de `tyrus:math` son valores `f64` resueltos en tiempo de compilación.

<!-- @include: ../../../.fragments/modules/math/constants/constants.md -->

| Constante | Valor | Descripción |
|-----------|-------|-------------|
| `PI` | `3.141592653589793` | Razón entre circunferencia y diámetro |
| `E` | `2.718281828459045` | Base de los logaritmos naturales |
| `LN2` | `0.6931471805599453` | Logaritmo natural de 2 |
| `LN10` | `2.302585092994046` | Logaritmo natural de 10 |
| `LOG2E` | `1.4426950408889634` | Logaritmo base 2 de E |
| `LOG10E` | `0.4342944819032518` | Logaritmo base 10 de E |
| `SQRT2` | `1.4142135623730951` | Raíz cuadrada de 2 |
| `EPSILON` | `2.220446049250313e-16` | Distancia de 1 al siguiente `f64` |
| `TAU` | `6.283185307179586` | Dos veces PI |
| `PHI` | `1.618033988749895` | Razón áurea |

::: tip Constantes de costo cero
Estas constantes se emiten como globales inicializadas en el binario generado. Importarlas no añade overhead de runtime más allá de leer un valor global.
:::
