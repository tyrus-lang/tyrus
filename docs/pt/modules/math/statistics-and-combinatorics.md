---
title: Estatística e combinatória
description: Funções estatísticas e combinatórias exportadas por tyrus:math.
---

# Estatística e combinatória

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`mean`](#mean) | `(values: f64[]) => f64` | Média aritmética; `0.0` para array vazia |
| [`median`](#median) | `(values: f64[]) => f64` | Mediana; `0.0` para array vazia |
| [`mode`](#mode) | `(values: f64[]) => f64` | Valor mais frequente; `0.0` para array vazia |
| [`variance`](#variance) | `(values: f64[]) => f64` | Variância populacional; `0.0` para array vazia |
| [`stdDev`](#stddev) | `(values: f64[]) => f64` | Desvio padrão populacional; `0.0` para array vazia |
| [`percentile`](#percentile) | `(p: f64, values: f64[]) => f64` | Percentil `p` (0-100) com interpolação linear; `0.0` para array vazia |
| [`factorial`](#factorial) | `(n: u128) => u128` | `n!` |
| [`permutation`](#permutation) | `(n: u128, k: u128) => u128` | `n! / (n - k)!` |
| [`combination`](#combination) | `(n: u128, k: u128) => u128` | `n! / (k! * (n - k)!)` |

#### `mean()`

Retorna a média aritmética de um array de floats.

<SignatureBlock label="Assinatura">mean(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-mean.md -->

#### `median()`

Retorna a mediana de um array de floats.

<SignatureBlock label="Assinatura">median(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-median.md -->

#### `mode()`

Retorna o valor mais frequente em um array.

<SignatureBlock label="Assinatura">mode(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-mode.md -->

#### `variance()`

Retorna a variância populacional de um array de floats.

<SignatureBlock label="Assinatura">variance(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-variance.md -->

#### `stdDev()`

Retorna o desvio padrão populacional de um array de floats.

<SignatureBlock label="Assinatura">stdDev(values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-stdDev.md -->

#### `percentile()`

Retorna o percentil `p` de um array usando interpolação linear.

<SignatureBlock label="Assinatura">percentile(p: f64, values: f64[]): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-percentile.md -->

#### `factorial()`

Retorna o fatorial de `n` (`n!`).

<SignatureBlock label="Assinatura">factorial(n: u128): u128</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-factorial.md -->

#### `permutation()`

Retorna o número de permutações de `k` elementos de `n` (`n! / (n - k)!`).

<SignatureBlock label="Assinatura">permutation(n: u128, k: u128): u128</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-permutation.md -->

#### `combination()`

Retorna o número de combinações de `k` elementos de `n` (`n! / (k! * (n - k)!)`).

<SignatureBlock label="Assinatura">combination(n: u128, k: u128): u128</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/statistics-combination.md -->
