---
title: Estadística y combinatoria
description: Funciones estadísticas y combinatorias exportadas por tyrus:math.
---

# Estadística y combinatoria

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`mean`](#mean) | `(values: f64[]) => f64` | Media aritmética; `0.0` para array vacía |
| [`median`](#median) | `(values: f64[]) => f64` | Mediana; `0.0` para array vacía |
| [`mode`](#mode) | `(values: f64[]) => f64` | Valor más frecuente; `0.0` para array vacía |
| [`variance`](#variance) | `(values: f64[]) => f64` | Varianza poblacional; `0.0` para array vacía |
| [`stdDev`](#stddev) | `(values: f64[]) => f64` | Desviación estándar poblacional; `0.0` para array vacía |
| [`percentile`](#percentile) | `(p: f64, values: f64[]) => f64` | Percentil `p` (0-100) con interpolación lineal; `0.0` para array vacía |
| [`factorial`](#factorial) | `(n: u128) => u128` | `n!` |
| [`permutation`](#permutation) | `(n: u128, k: u128) => u128` | `n! / (n - k)!` |
| [`combination`](#combination) | `(n: u128, k: u128) => u128` | `n! / (k! * (n - k)!)` |

#### `mean()`

Devuelve la media aritmética de un array de floats.

<SignatureBlock label="Firma">mean(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-mean.md -->

#### `median()`

Devuelve la mediana de un array de floats.

<SignatureBlock label="Firma">median(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-median.md -->

#### `mode()`

Devuelve el valor más frecuente en un array.

<SignatureBlock label="Firma">mode(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-mode.md -->

#### `variance()`

Devuelve la varianza poblacional de un array de floats.

<SignatureBlock label="Firma">variance(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-variance.md -->

#### `stdDev()`

Devuelve la desviación estándar poblacional de un array de floats.

<SignatureBlock label="Firma">stdDev(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-stdDev.md -->

#### `percentile()`

Devuelve el percentil `p` de un array usando interpolación lineal.

<SignatureBlock label="Firma">percentile(p: f64, values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-percentile.md -->

#### `factorial()`

Devuelve el factorial de `n` (`n!`).

<SignatureBlock label="Firma">factorial(n: u128): u128</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-factorial.md -->

#### `permutation()`

Devuelve el número de permutaciones de `k` elementos de `n` (`n! / (n - k)!`).

<SignatureBlock label="Firma">permutation(n: u128, k: u128): u128</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-permutation.md -->

#### `combination()`

Devuelve el número de combinaciones de `k` elementos de `n` (`n! / (k! * (n - k)!)`).

<SignatureBlock label="Firma">combination(n: u128, k: u128): u128</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-combination.md -->
