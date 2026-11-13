---
title: simdSub
description: Subtração vetorizada elemento a elemento de arrays numéricos do mesmo tamanho.
---

# simdSub

`simdSub` subtrai cada elemento do array à direita do elemento correspondente do array à esquerda e retorna um novo array em `Result<T[], SimdError>`.

<SignatureBlock label="Assinatura">simdSub&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/simd/sub.md -->

Os tipos aceitos são `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32` e `f64`. A subtração de inteiros com e sem sinal faz wrap por padrão na largura do elemento; por exemplo, subtrair `1` de `0` como `u8` resulta em `255`. Ainda é obrigatório que literais inteiros caibam no tipo declarado para os elementos. Diferenças de tamanho dinâmico e falhas de alocação do resultado retornam `SimdError`. Arrays fixos com comprimentos diferentes são rejeitados com `E0308` em compilação.

## Erros

::: danger error
<!-- @include: ../../../.fragments/modules/simd/sub-length-mismatch.md -->
:::

::: danger error
<!-- @include: ../../../.fragments/modules/simd/sub-type-mismatch.md -->
:::
