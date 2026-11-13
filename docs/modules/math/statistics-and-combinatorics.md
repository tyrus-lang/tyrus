---
title: Statistics and combinatorics
description: Statistical and combinatorial math functions exported by tyrus:math.
---

# Statistics and combinatorics

| Function | Signature | Description |
|----------|-----------|-------------|
| [`mean`](#mean) | `(values: f64[]) => f64` | Arithmetic mean; `0.0` for empty array |
| [`median`](#median) | `(values: f64[]) => f64` | Median; `0.0` for empty array |
| [`mode`](#mode) | `(values: f64[]) => f64` | Most frequent value; `0.0` for empty array |
| [`variance`](#variance) | `(values: f64[]) => f64` | Population variance; `0.0` for empty array |
| [`stdDev`](#stddev) | `(values: f64[]) => f64` | Population standard deviation; `0.0` for empty array |
| [`percentile`](#percentile) | `(p: f64, values: f64[]) => f64` | `p`-th percentile (0-100) with linear interpolation; `0.0` for empty array |
| [`factorial`](#factorial) | `(n: u128) => u128` | `n!` |
| [`permutation`](#permutation) | `(n: u128, k: u128) => u128` | `n! / (n - k)!` |
| [`combination`](#combination) | `(n: u128, k: u128) => u128` | `n! / (k! * (n - k)!)` |

#### `mean()`

Returns the arithmetic mean of an array of floats.

<SignatureBlock label="Signature">mean(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/statistics-mean.md -->

#### `median()`

Returns the median of an array of floats.

<SignatureBlock label="Signature">median(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/statistics-median.md -->

#### `mode()`

Returns the most frequent value in an array.

<SignatureBlock label="Signature">mode(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/statistics-mode.md -->

#### `variance()`

Returns the population variance of an array of floats.

<SignatureBlock label="Signature">variance(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/statistics-variance.md -->

#### `stdDev()`

Returns the population standard deviation of an array of floats.

<SignatureBlock label="Signature">stdDev(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/statistics-stdDev.md -->

#### `percentile()`

Returns the `p`-th percentile of an array using linear interpolation.

<SignatureBlock label="Signature">percentile(p: f64, values: f64[]): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/statistics-percentile.md -->

#### `factorial()`

Returns the factorial of `n` (`n!`).

<SignatureBlock label="Signature">factorial(n: u128): u128</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/statistics-factorial.md -->

#### `permutation()`

Returns the number of permutations of `k` elements from `n` (`n! / (n - k)!`).

<SignatureBlock label="Signature">permutation(n: u128, k: u128): u128</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/statistics-permutation.md -->

#### `combination()`

Returns the number of combinations of `k` elements from `n` (`n! / (k! * (n - k)!)`).

<SignatureBlock label="Signature">combination(n: u128, k: u128): u128</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/statistics-combination.md -->
