---
title: Trigonometria
description: Funções trigonométricas exportadas por tyrus:math.
---

# Trigonometria

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`sin`](#sin) | `(x: f64) => f64` | Seno |
| [`cos`](#cos) | `(x: f64) => f64` | Cosseno |
| [`tan`](#tan) | `(x: f64) => f64` | Tangente |
| [`asin`](#asin) | `(x: f64) => f64` | Arco seno |
| [`acos`](#acos) | `(x: f64) => f64` | Arco cosseno |
| [`atan`](#atan) | `(x: f64) => f64` | Arco tangente |
| [`atan2`](#atan2) | `(y: f64, x: f64) => f64` | Arco tangente de `y/x` |
| [`degToRad`](#degtorad) | `(degrees: f64) => f64` | Converte graus para radianos |
| [`radToDeg`](#radtodeg) | `(radians: f64) => f64` | Converte radianos para graus |

#### `sin()`

Retorna o seno de um ângulo em radianos.

<SignatureBlock label="Assinatura">sin(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-sin.md -->

#### `cos()`

Retorna o cosseno de um ângulo em radianos.

<SignatureBlock label="Assinatura">cos(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-cos.md -->

#### `tan()`

Retorna a tangente de um ângulo em radianos.

<SignatureBlock label="Assinatura">tan(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-tan.md -->

#### `asin()`

Retorna o arco seno (seno inverso) de um valor em radianos.

<SignatureBlock label="Assinatura">asin(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-asin.md -->

#### `acos()`

Retorna o arco cosseno (cosseno inverso) de um valor em radianos.

<SignatureBlock label="Assinatura">acos(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-acos.md -->

#### `atan()`

Retorna o arco tangente (tangente inversa) de um valor em radianos.

<SignatureBlock label="Assinatura">atan(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-atan.md -->

#### `atan2()`

Retorna o arco tangente de `y/x`, lidando corretamente com o quadrante.

<SignatureBlock label="Assinatura">atan2(y: f64, x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-atan2.md -->

#### `degToRad()`

Converte um ângulo de graus para radianos.

<SignatureBlock label="Assinatura">degToRad(degrees: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-degToRad.md -->

#### `radToDeg()`

Converte um ângulo de radianos para graus.

<SignatureBlock label="Assinatura">radToDeg(radians: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/trigonometry-radToDeg.md -->
