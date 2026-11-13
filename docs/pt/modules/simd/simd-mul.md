---
title: simdMul
description: Multiplicação vetorizada elemento a elemento de arrays numéricos do mesmo tamanho.
---

# simdMul

`simdMul` multiplica os elementos correspondentes de dois arrays do mesmo tamanho e retorna um novo array em `Result<T[], SimdError>`.

<SignatureBlock label="Assinatura">simdMul&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/simd/mul.md -->

Os tipos aceitos são `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32` e `f64`. A multiplicação de inteiros com e sem sinal faz wrap por padrão na largura do elemento; por exemplo, multiplicar `255` por `2` como `u8` resulta em `254`. Ainda é obrigatório que literais inteiros caibam no tipo declarado para os elementos. Diferenças de tamanho dinâmico e falhas de alocação do resultado retornam `SimdError`. Arrays fixos com comprimentos diferentes são rejeitados com `E0308` em compilação.

## Erros

::: danger error
<!-- @include: ../../../.fragments/modules/simd/mul-length-mismatch.md -->
:::

::: danger error
<!-- @include: ../../../.fragments/modules/simd/mul-type-mismatch.md -->
:::
