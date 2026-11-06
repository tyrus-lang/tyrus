---
title: Geometría y utilidades numéricas
description: Funciones de geometría y utilidades numéricas exportadas por tyrus:math.
---

# Geometría y utilidades numéricas

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`hypot`](#hypot) `(x: f64, y: f64) => f64` | Norma Euclidiana `sqrt(x^2 + y^2)` |
| [`toPolar`](#topolar) `(x: f64, y: f64) => [f64, f64]` | Convierte Cartesianas a polares `(r, theta)` |
| [`toCartesian`](#tocartesian) `(r: f64, theta: f64) => [f64, f64]` | Convierte polares a Cartesianas `(x, y)` |
| [`copysign`](#copysign) `(x: f64, y: f64) => f64` | Copia signo de `y` a `x` |
| [`fmod`](#fmod) `(x: f64, y: f64) => f64` | Resto en punto flotante de `x / y` |
| [`logBase`](#logbase) `(x: f64, base: f64) => f64` | Logaritmo de `x` en la `base` dada |
| [`powi`](#powi) `(x: f64, exp: i32) => f64` | `x` elevado a la potencia entera `exp` |

#### `hypot()`

Devuelve la norma Euclidiana (hipotenusa) `sqrt(x^2 + y^2)` sin overflow intermedio.

<SignatureBlock label="Firma">hypot(x: f64, y: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-hypot.md -->

#### `toPolar()`

Convierte coordenadas Cartesianas `(x, y)` a coordenadas polares `(r, theta)`.

<SignatureBlock label="Firma">toPolar(x: f64, y: f64): [f64, f64]</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-toPolar.md -->

#### `toCartesian()`

Convierte coordenadas polares `(r, theta)` a coordenadas Cartesianas `(x, y)`.

<SignatureBlock label="Firma">toCartesian(r: f64, theta: f64): [f64, f64]</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-toCartesian.md -->

#### `copysign()`

Devuelve un valor con la magnitud de `x` y el signo de `y`.

<SignatureBlock label="Firma">copysign(x: f64, y: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-copysign.md -->

#### `fmod()`

Devuelve el resto en punto flotante de `x / y`.

<SignatureBlock label="Firma">fmod(x: f64, y: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-fmod.md -->

#### `logBase()`

Devuelve el logaritmo de `x` en la `base` dada.

<SignatureBlock label="Firma">logBase(x: f64, base: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-logBase.md -->

#### `powi()`

Devuelve `x` elevado a la potencia entera `exp`.

<SignatureBlock label="Firma">powi(x: f64, exp: i32): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/geometry-powi.md -->
