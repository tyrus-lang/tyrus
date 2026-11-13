---
title: Exponenciales y logaritmos
description: Funciones exponenciales y logarítmicas exportadas por tyrus:math.
---

# Exponenciales y logaritmos

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`exp`](#exp) | `(x: f64) => f64` | `e^x` |
| [`log`](#log) | `(x: f64) => f64` | Logaritmo natural |
| [`log10`](#log10) | `(x: f64) => f64` | Logaritmo base 10 |
| [`log2`](#log2) | `(x: f64) => f64` | Logaritmo base 2 |
| [`pow`](#pow) | `(base: f64, exp: f64) => f64` | Potencia |

#### `exp()`

Devuelve el número de Euler `e` elevado a la potencia de `x`.

<SignatureBlock label="Firma">exp(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/exponentials-exp.md -->

#### `log()`

Devuelve el logaritmo natural (base `e`) de `x`.

<SignatureBlock label="Firma">log(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/exponentials-log.md -->

#### `log10()`

Devuelve el logaritmo base 10 de `x`.

<SignatureBlock label="Firma">log10(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/exponentials-log10.md -->

#### `log2()`

Devuelve el logaritmo base 2 de `x`.

<SignatureBlock label="Firma">log2(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/exponentials-log2.md -->

#### `pow()`

Devuelve `base` elevado a la potencia de `exp`.

<SignatureBlock label="Firma">pow(base: f64, exp: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/exponentials-pow.md -->
