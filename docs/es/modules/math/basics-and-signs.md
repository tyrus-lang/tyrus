---
title: Básicas y signos
description: Funciones matemáticas básicas exportadas por tyrus:math.
---

# Básicas y signos

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`abs`](#abs) `(x: f64) => f64` | Valor absoluto |
| [`sign`](#sign) `(x: f64) => f64` | Signo (`1`, `-1` o `0`) |
| [`sqrt`](#sqrt) `(x: f64) => f64` | Raíz cuadrada |
| [`cbrt`](#cbrt) `(x: f64) => f64` | Raíz cúbica |

#### `abs()`

Devuelve el valor absoluto de un número.

<SignatureBlock label="Firma">abs(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/basics-abs.md -->

#### `sign()`

Devuelve el signo de un número: `1.0` para positivo, `-1.0` para negativo o `0.0` para cero.

<SignatureBlock label="Firma">sign(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/basics-sign.md -->

#### `sqrt()`

Devuelve la raíz cuadrada de un número. Entradas negativas causan panic en runtime.

<SignatureBlock label="Firma">sqrt(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/basics-sqrt.md -->

#### `cbrt()`

Devuelve la raíz cúbica de un número. Entradas negativas son válidas.

<SignatureBlock label="Firma">cbrt(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/basics-cbrt.md -->
