---
title: Utilitários avançados
description: Funções utilitárias avançadas exportadas por tyrus:math.
---

# Utilitários avançados

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`approxEqual`](#approxequal) `(a: f64, b: f64, tolerance?: f64) => boolean` | Compara floats dentro da tolerância |
| [`lerp`](#lerp) `(start: f64, end: f64, amount: f64) => f64` | Interpolação linear |
| [`inverseLerp`](#inverselerp) `(start: f64, end: f64, value: f64) => f64` | Interpolação linear inversa |
| [`remap`](#remap) `(value: f64, inMin: f64, inMax: f64, outMin: f64, outMax: f64) => f64` | Remapeia valor entre escalas |
| [`smoothstep`](#smoothstep) `(edge0: f64, edge1: f64, x: f64) => f64` | Interpolação suave de Hermite |
| [`gcd`](#gcd) `(a: u32, b: u32) => u32` | Máximo divisor comum |
| [`lcm`](#lcm) `(a: u32, b: u32) => u32` | Mínimo múltiplo comum |
| [`randomRange`](#randomrange) `(min: f64, max: f64) => f64` | Float aleatório no intervalo semiaberto |

#### `approxEqual()`

Verifica se dois floats são iguais dentro de uma tolerância opcional.

<SignatureBlock label="Assinatura">approxEqual(a: f64, b: f64, tolerance?: f64): boolean</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-approxEqual.md -->

#### `lerp()`

Interpola linearmente entre dois valores.

<SignatureBlock label="Assinatura">lerp(start: f64, end: f64, amount: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-lerp.md -->

#### `inverseLerp()`

Retorna a fração normalizada de onde um valor está entre dois extremos.

<SignatureBlock label="Assinatura">inverseLerp(start: f64, end: f64, value: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-inverseLerp.md -->

#### `remap()`

Remapeia um valor de uma escala para outra.

<SignatureBlock label="Assinatura">remap(value: f64, inMin: f64, inMax: f64, outMin: f64, outMax: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-remap.md -->

#### `smoothstep()`

Retorna interpolação suave de Hermite entre duas bordas.

<SignatureBlock label="Assinatura">smoothstep(edge0: f64, edge1: f64, x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-smoothstep.md -->

#### `gcd()`

Retorna o máximo divisor comum de dois inteiros sem sinal.

<SignatureBlock label="Assinatura">gcd(a: u32, b: u32): u32</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-gcd.md -->

#### `lcm()`

Retorna o mínimo múltiplo comum de dois inteiros sem sinal.

<SignatureBlock label="Assinatura">lcm(a: u32, b: u32): u32</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-lcm.md -->

#### `randomRange()`

Retorna um número floating-point aleatório no intervalo semiaberto `[min, max)`.

<SignatureBlock label="Assinatura">randomRange(min: f64, max: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/advanced-randomRange.md -->
