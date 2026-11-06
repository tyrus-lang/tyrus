---
title: Radix
description: Formatação e análise de inteiros nas bases 2, 8, 10 e 16.
---

# Radix

<SignatureBlock label="Assinatura">radix.format(value: i64, base: 2 | 8 | 10 | 16): string</SignatureBlock>

<SignatureBlock label="Assinatura">radix.parseI64(text: string, base: 2 | 8 | 10 | 16): Result&lt;i64, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Assinatura">radix.parseU64(text: string, base: 2 | 8 | 10 | 16): Result&lt;u64, EncodingError&gt;</SignatureBlock>

Importe `radix` de `tyrus:encoding`. `format(value, base)` retorna o valor na base informada usando dígitos minúsculos, com `-` inicial para valores negativos e magnitude sem sinal. As únicas bases suportadas são 2, 8, 10 e 16, e `base` tem o tipo `2 | 8 | 10 | 16`, então qualquer outra base é um erro de tipo em compilação e `format` nunca lança. `parseI64` e `parseU64` revertem a formatação: aceitam somente dígitos minúsculos, rejeitam qualquer prefixo e nunca arredondam. `parseI64` permite um único `-` inicial, rejeita `+` e estoura com `ERR_ENCODING_INPUT_TOO_LARGE`; `parseU64` rejeita qualquer sinal.

<!-- @include: ../../../.fragments/modules/encoding/radix-usage.md -->

`format` e `parse` são inversos exatos para todo valor representável: nenhum sinal é inventado, nenhum dígito é normalizado e cada base tem exatamente uma grafia canônica que faz round-trip. Dígitos em maiúsculas como `FF` são rejeitados, e uma base diferente de 2, 8, 10 ou 16 não passa na checagem de tipos.