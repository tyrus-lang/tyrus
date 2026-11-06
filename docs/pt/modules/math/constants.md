---
title: Constantes
description: Constantes matemáticas exportadas por tyrus:math.
---

# Constantes

Todas as constantes de `tyrus:math` são valores `f64` resolvidos em tempo de compilação.

<!-- @include: ../../../.fragments/modules/math/constants/constants.md -->

| Constante | Valor | Descrição |
|-----------|-------|-----------|
| `PI` | `3.141592653589793` | Razão entre circunferência e diâmetro |
| `E` | `2.718281828459045` | Base dos logaritmos naturais |
| `LN2` | `0.6931471805599453` | Logaritmo natural de 2 |
| `LN10` | `2.302585092994046` | Logaritmo natural de 10 |
| `LOG2E` | `1.4426950408889634` | Logaritmo base 2 de E |
| `LOG10E` | `0.4342944819032518` | Logaritmo base 10 de E |
| `SQRT2` | `1.4142135623730951` | Raiz quadrada de 2 |
| `EPSILON` | `2.220446049250313e-16` | Distância de 1 ao próximo `f64` |
| `TAU` | `6.283185307179586` | Duas vezes PI |
| `PHI` | `1.618033988749895` | Razão áurea |

::: tip Constantes de custo zero
Estas constantes são emitidas como globais inicializadas no binário gerado. Importá-las não adiciona overhead de runtime além da leitura de um valor global.
:::
