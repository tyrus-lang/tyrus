---
title: simdAdd
description: Soma vetorizada elemento a elemento de arrays numéricos do mesmo tamanho.
---

# simdAdd

`simdAdd` soma os elementos correspondentes de dois arrays do mesmo tamanho e retorna um novo array em `Result<T[], SimdError>`.

<SignatureBlock label="Assinatura">simdAdd&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/simd/add.md -->

Se os dois argumentos forem arrays fixos com comprimentos diferentes, o compilador informa `E0308`. Se os tamanhos só forem conhecidos em runtime, `simdAdd` retorna `Err` com o código `ERR_SIMD_LENGTH_MISMATCH`. Se não for possível alocar o array de resultado, retorna `ERR_SIMD_OUT_OF_MEMORY`. Essas falhas recuperáveis não provocam panic. Todas as lanes inteiras usam wrap por padrão. Por exemplo, `120 + 120` como `i8` resulta em `-16`, e `255 + 1` como `u8` resulta em `0`, não `256`. Ainda é obrigatório que literais inteiros caibam no tipo declarado para os elementos. Os tipos aceitos são `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32` e `f64`; `i128`, `u128`, `f128`, `isize`, `usize` e `decimal` não são aceitos. O compilador escolhe as larguras vetoriais e trata os elementos restantes com segurança; a API não promete um ganho de velocidade fixo entre alvos.

Arrays de tamanho fixo também funcionam como argumentos diretos. O tamanho é conhecido em compilação, e a operação SIMD lê seus elementos sem alocar antes um array dinâmico de entrada.

## Erros

::: danger error
<!-- @include: ../../../.fragments/modules/simd/add-length-mismatch.md -->
:::

::: danger error
<!-- @include: ../../../.fragments/modules/simd/add-type-mismatch.md -->
:::
