---
title: Utilidades avanzadas
description: Funciones de utilidad avanzadas exportadas por tyrus:math.
---

# Utilidades avanzadas

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`approxEqual`](#approxequal) `(a: f64, b: f64, tolerance?: f64) => boolean` | Compara floats dentro de la tolerancia |
| [`lerp`](#lerp) `(start: f64, end: f64, amount: f64) => f64` | Interpolación lineal |
| [`inverseLerp`](#inverselerp) `(start: f64, end: f64, value: f64) => f64` | Interpolación lineal inversa |
| [`remap`](#remap) `(value: f64, inMin: f64, inMax: f64, outMin: f64, outMax: f64) => f64` | Remapea valor entre escalas |
| [`smoothstep`](#smoothstep) `(edge0: f64, edge1: f64, x: f64) => f64` | Interpolación suave de Hermite |
| [`gcd`](#gcd) `(a: u32, b: u32) => u32` | Máximo divisor común |
| [`lcm`](#lcm) `(a: u32, b: u32) => u32` | Mínimo múltiplo común |
| [`randomRange`](#randomrange) `(min: f64, max: f64) => f64` | Float aleatorio en el intervalo semiabierto |

#### `approxEqual()`

Verifica si dos floats son iguales dentro de una tolerancia opcional.

<SignatureBlock label="Firma">approxEqual(a: f64, b: f64, tolerance?: f64): boolean</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-approxEqual.md -->

#### `lerp()`

Interpola linealmente entre dos valores.

<SignatureBlock label="Firma">lerp(start: f64, end: f64, amount: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-lerp.md -->

#### `inverseLerp()`

Devuelve la fracción normalizada de donde un valor está entre dos extremos.

<SignatureBlock label="Firma">inverseLerp(start: f64, end: f64, value: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-inverseLerp.md -->

#### `remap()`

Remapea un valor de un rango a otro.

<SignatureBlock label="Firma">remap(value: f64, inMin: f64, inMax: f64, outMin: f64, outMax: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-remap.md -->

#### `smoothstep()`

Devuelve interpolación suave de Hermite entre dos bordes.

<SignatureBlock label="Firma">smoothstep(edge0: f64, edge1: f64, x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-smoothstep.md -->

#### `gcd()`

Devuelve el máximo divisor común de dos enteros sin signo.

<SignatureBlock label="Firma">gcd(a: u32, b: u32): u32</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-gcd.md -->

#### `lcm()`

Devuelve el mínimo múltiplo común de dos enteros sin signo.

<SignatureBlock label="Firma">lcm(a: u32, b: u32): u32</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-lcm.md -->

#### `randomRange()`

Devuelve un número floating-point aleatorio en el intervalo semiabierto `[min, max)`.

<SignatureBlock label="Firma">randomRange(min: f64, max: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-randomRange.md -->
