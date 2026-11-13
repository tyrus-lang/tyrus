---
title: Seleção e baixo nível
description: Funções de seleção e baixo nível exportadas por tyrus:math.
---

# Seleção e baixo nível

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`max`](#max) | `(a: f64, b: f64) => f64` | Máximo |
| [`min`](#min) | `(a: f64, b: f64) => f64` | Mínimo |
| [`clamp`](#clamp) | `(value: f64, min: f64, max: f64) => f64` | Limita valor ao intervalo |
| [`clampMin`](#clampmin) | `(value: f64, min: f64) => f64` | Garante valor mínimo |
| [`clampMax`](#clampmax) | `(value: f64, max: f64) => f64` | Garante valor máximo |
| [`imul`](#imul) | `(a: i32, b: i32) => i32` | Multiplicação C-style 32-bit |
| [`clz32`](#clz32) | `(x: i32) => i32` | Conta bits zero à esquerda em inteiro 32-bit |

#### `max()`

Retorna o maior entre dois números.

<SignatureBlock label="Assinatura">max(a: f64, b: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-max.md -->

#### `min()`

Retorna o menor entre dois números.

<SignatureBlock label="Assinatura">min(a: f64, b: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-min.md -->

#### `clamp()`

Restringe um valor a um intervalo dado.

<SignatureBlock label="Assinatura">clamp(value: f64, min: f64, max: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-clamp.md -->

#### `clampMin()`

Garante um valor mínimo.

<SignatureBlock label="Assinatura">clampMin(value: f64, min: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-clampMin.md -->

#### `clampMax()`

Garante um valor máximo.

<SignatureBlock label="Assinatura">clampMax(value: f64, max: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-clampMax.md -->

#### `imul()`

Realiza multiplicação inteira C-style de 32 bits, truncando o overflow.

<SignatureBlock label="Assinatura">imul(a: i32, b: i32): i32</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-imul.md -->

#### `clz32()`

Conta os bits zero à esquerda em um inteiro de 32 bits.

<SignatureBlock label="Assinatura">clz32(x: i32): i32</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-clz32.md -->
