---
title: UTF-8
description: Conversão de bytes UTF-8.
---

# UTF-8

<SignatureBlock label="Assinatura">utf8.encode(text: string): u8[]</SignatureBlock>

<SignatureBlock label="Assinatura">utf8.decode(bytes: u8[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Assinatura">utf8.decodeLossy(bytes: u8[]): string</SignatureBlock>

Importe `utf8` de `tyrus:encoding`. `encode(text)` retorna os bytes UTF-8 de uma string. `decode(bytes)` aceita somente UTF-8 bem-formado e retorna `Err(EncodingError)` nos demais casos. `decodeLossy(bytes)` substitui cada sequência inválida por U+FFFD.

<!-- @include: ../../../.fragments/modules/encoding/utf8/utf8.md -->

UTF-8 não tem forma little-endian nem big-endian: seus valores codificados são sequências de bytes definidos individualmente. Endianness só se aplica quando uma codificação usa unidades de código com múltiplos bytes, como UTF-16.
