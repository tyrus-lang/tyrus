---
title: Básicas e sinais
description: Funções básicas de matemática exportadas por tyrus:math.
---

# Básicas e sinais

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`abs`](#abs) `(x: f64) => f64` | Valor absoluto |
| [`sign`](#sign) `(x: f64) => f64` | Sinal (`1`, `-1` ou `0`) |
| [`sqrt`](#sqrt) `(x: f64) => f64` | Raiz quadrada |
| [`cbrt`](#cbrt) `(x: f64) => f64` | Raiz cúbica |

#### `abs()`

Retorna o valor absoluto de um número.

<SignatureBlock label="Assinatura">abs(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/basics-abs.md -->

#### `sign()`

Retorna o sinal de um número: `1.0` para positivo, `-1.0` para negativo ou `0.0` para zero.

<SignatureBlock label="Assinatura">sign(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/basics-sign.md -->

#### `sqrt()`

Retorna a raiz quadrada de um número. Entradas negativas causam panic em runtime.

<SignatureBlock label="Assinatura">sqrt(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/basics-sqrt.md -->

#### `cbrt()`

Retorna a raiz cúbica de um número. Entradas negativas são válidas.

<SignatureBlock label="Assinatura">cbrt(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/basics-cbrt.md -->
