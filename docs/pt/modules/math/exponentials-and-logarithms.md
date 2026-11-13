---
title: Exponenciais e logaritmos
description: Funções exponenciais e logarítmicas exportadas por tyrus:math.
---

# Exponenciais e logaritmos

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`exp`](#exp) | `(x: f64) => f64` | `e^x` |
| [`log`](#log) | `(x: f64) => f64` | Logaritmo natural |
| [`log10`](#log10) | `(x: f64) => f64` | Logaritmo base 10 |
| [`log2`](#log2) | `(x: f64) => f64` | Logaritmo base 2 |
| [`pow`](#pow) | `(base: f64, exp: f64) => f64` | Potência |

#### `exp()`

Retorna o número de Euler `e` elevado à potência de `x`.

<SignatureBlock label="Assinatura">exp(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/exponentials-exp.md -->

#### `log()`

Retorna o logaritmo natural (base `e`) de `x`.

<SignatureBlock label="Assinatura">log(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/exponentials-log.md -->

#### `log10()`

Retorna o logaritmo base 10 de `x`.

<SignatureBlock label="Assinatura">log10(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/exponentials-log10.md -->

#### `log2()`

Retorna o logaritmo base 2 de `x`.

<SignatureBlock label="Assinatura">log2(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/exponentials-log2.md -->

#### `pow()`

Retorna `base` elevado à potência de `exp`.

<SignatureBlock label="Assinatura">pow(base: f64, exp: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/exponentials-pow.md -->
