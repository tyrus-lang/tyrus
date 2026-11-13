---
title: Trigonometría
description: Funciones trigonométricas exportadas por tyrus:math.
---

# Trigonometría

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`sin`](#sin) | `(x: f64) => f64` | Seno |
| [`cos`](#cos) | `(x: f64) => f64` | Coseno |
| [`tan`](#tan) | `(x: f64) => f64` | Tangente |
| [`asin`](#asin) | `(x: f64) => f64` | Arco seno |
| [`acos`](#acos) | `(x: f64) => f64` | Arco coseno |
| [`atan`](#atan) | `(x: f64) => f64` | Arco tangente |
| [`atan2`](#atan2) | `(y: f64, x: f64) => f64` | Arco tangente de `y/x` |
| [`degToRad`](#degtorad) | `(degrees: f64) => f64` | Convierte grados a radianes |
| [`radToDeg`](#radtodeg) | `(radians: f64) => f64` | Convierte radianes a grados |

#### `sin()`

Devuelve el seno de un ángulo en radianes.

<SignatureBlock label="Firma">sin(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-sin.md -->

#### `cos()`

Devuelve el coseno de un ángulo en radianes.

<SignatureBlock label="Firma">cos(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-cos.md -->

#### `tan()`

Devuelve la tangente de un ángulo en radianes.

<SignatureBlock label="Firma">tan(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-tan.md -->

#### `asin()`

Devuelve el arco seno (seno inverso) de un valor en radianes.

<SignatureBlock label="Firma">asin(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-asin.md -->

#### `acos()`

Devuelve el arco coseno (coseno inverso) de un valor en radianes.

<SignatureBlock label="Firma">acos(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-acos.md -->

#### `atan()`

Devuelve el arco tangente (tangente inversa) de un valor en radianes.

<SignatureBlock label="Firma">atan(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-atan.md -->

#### `atan2()`

Devuelve el arco tangente de `y/x`, manejando correctamente el cuadrante.

<SignatureBlock label="Firma">atan2(y: f64, x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-atan2.md -->

#### `degToRad()`

Convierte un ángulo de grados a radianes.

<SignatureBlock label="Firma">degToRad(degrees: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-degToRad.md -->

#### `radToDeg()`

Convierte un ángulo de radianes a grados.

<SignatureBlock label="Firma">radToDeg(radians: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-radToDeg.md -->
