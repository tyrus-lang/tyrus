---
title: Geometria e utilitários numéricos
description: Funções de geometria e utilitários numéricos exportadas por tyrus:math.
---

# Geometria e utilitários numéricos

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`hypot`](#hypot) `(x: f64, y: f64) => f64` | Norma Euclidiana `sqrt(x^2 + y^2)` |
| [`toPolar`](#topolar) `(x: f64, y: f64) => [f64, f64]` | Converte Cartesianas para polares `(r, theta)` |
| [`toCartesian`](#tocartesian) `(r: f64, theta: f64) => [f64, f64]` | Converte polares para Cartesianas `(x, y)` |
| [`copysign`](#copysign) `(x: f64, y: f64) => f64` | Copia sinal de `y` para `x` |
| [`fmod`](#fmod) `(x: f64, y: f64) => f64` | Resto floating-point de `x / y` |
| [`logBase`](#logbase) `(x: f64, base: f64) => f64` | Logaritmo de `x` na `base` dada |
| [`powi`](#powi) `(x: f64, exp: i32) => f64` | `x` elevado à potência inteira `exp` |

#### `hypot()`

Retorna a norma Euclidiana (hipotenusa) `sqrt(x^2 + y^2)` sem overflow intermediário.

<SignatureBlock label="Assinatura">hypot(x: f64, y: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-hypot.md -->

#### `toPolar()`

Converte coordenadas Cartesianas `(x, y)` para coordenadas polares `(r, theta)`.

<SignatureBlock label="Assinatura">toPolar(x: f64, y: f64): [f64, f64]</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-toPolar.md -->

#### `toCartesian()`

Converte coordenadas polares `(r, theta)` para coordenadas Cartesianas `(x, y)`.

<SignatureBlock label="Assinatura">toCartesian(r: f64, theta: f64): [f64, f64]</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-toCartesian.md -->

#### `copysign()`

Retorna um valor com a magnitude de `x` e o sinal de `y`.

<SignatureBlock label="Assinatura">copysign(x: f64, y: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-copysign.md -->

#### `fmod()`

Retorna o resto floating-point de `x / y`.

<SignatureBlock label="Assinatura">fmod(x: f64, y: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-fmod.md -->

#### `logBase()`

Retorna o logaritmo de `x` na `base` dada.

<SignatureBlock label="Assinatura">logBase(x: f64, base: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-logBase.md -->

#### `powi()`

Retorna `x` elevado a uma potência inteira `exp`.

<SignatureBlock label="Assinatura">powi(x: f64, exp: i32): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-powi.md -->
