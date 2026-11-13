---
title: Arredondamento
description: Funções de arredondamento exportadas por tyrus:math.
---

# Arredondamento

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`floor`](#floor) | `(x: f64) => f64` | Arredonda para baixo |
| [`ceil`](#ceil) | `(x: f64) => f64` | Arredonda para cima |
| [`trunc`](#trunc) | `(x: f64) => f64` | Trunca em direção a zero |
| [`round`](#round) | `(x: f64) => f64` | Arredonda para o inteiro mais próximo |
| [`roundTo`](#roundto) | `(x: f64, decimals: i32) => f64` | Arredonda para N casas decimais |

#### `floor()`

Arredonda um número para baixo até o inteiro mais próximo.

<SignatureBlock label="Assinatura">floor(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/rounding-floor.md -->

#### `ceil()`

Arredonda um número para cima até o inteiro mais próximo.

<SignatureBlock label="Assinatura">ceil(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/rounding-ceil.md -->

#### `trunc()`

Trunca um número em direção a zero, removendo a parte fracionária.

<SignatureBlock label="Assinatura">trunc(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/rounding-trunc.md -->

#### `round()`

Arredonda um número para o inteiro mais próximo.

<SignatureBlock label="Assinatura">round(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/rounding-round.md -->

#### `roundTo()`

Arredonda um número para um número especificado de casas decimais.

<SignatureBlock label="Assinatura">roundTo(x: f64, decimals: i32): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/rounding-roundTo.md -->
