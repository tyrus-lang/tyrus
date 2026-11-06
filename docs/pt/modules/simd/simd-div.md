---
title: simdDiv
description: Divisão vetorizada elemento a elemento de arrays numéricos.
---

# simdDiv

`simdDiv` divide os elementos correspondentes e retorna um novo array em `Result<T[], SimdError>`.

<SignatureBlock label="Assinatura">simdDiv&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/simd/div.md -->

A divisão inteira trunca em direção a zero. Se qualquer lane inteira tiver divisor zero, o resultado será `ERR_SIMD_DIVISION_BY_ZERO`. O mínimo signed dividido por `-1` retorna `ERR_SIMD_INTEGER_OVERFLOW`, pois o quociente exato não cabe no tipo da lane. A operação verifica todas as lanes inteiras antes da divisão, então nenhuma lane inválida chega a uma divisão inteira. A divisão de ponto flutuante segue IEEE e retorna infinito ou NaN em `Ok` quando apropriado. Diferenças de tamanho dinâmico e falhas de alocação do resultado também retornam `SimdError`. Arrays de tamanho fixo podem ser passados diretamente; comprimentos fixos diferentes são rejeitados com `E0308` em compilação.

## Divisão de ponto flutuante por zero

A divisão de ponto flutuante por zero segue IEEE 754 e retorna estes valores em `Ok`: `1.0 / 0.0` produz `Infinity`, `-1.0 / 0.0` produz `-Infinity` e `0.0 / 0.0` produz `NaN`.

<!-- @include: ../../../.fragments/modules/simd/div-float.md -->

## Erros

::: danger error
Um divisor inteiro igual a zero retorna um erro recuperável:
<!-- @include: ../../../.fragments/modules/simd/div-zero-error.md -->
:::

::: danger error
O mínimo signed dividido por `-1` também retorna um erro recuperável:
<!-- @include: ../../../.fragments/modules/simd/div-overflow-error.md -->
:::
