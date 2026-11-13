---
title: UTF-8
description: Conversión de bytes UTF-8.
---

# UTF-8

<SignatureBlock label="Firma">utf8.encode(text: string): u8[]</SignatureBlock>

<SignatureBlock label="Firma">utf8.decode(bytes: u8[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Firma">utf8.decodeLossy(bytes: u8[]): string</SignatureBlock>

Importe `utf8` de `tyrus:encoding`. `encode(text)` devuelve los bytes UTF-8 de una string. `decode(bytes)` acepta solo UTF-8 bien formado y devuelve `Err(EncodingError)` en caso contrario. `decodeLossy(bytes)` sustituye cada secuencia no válida por U+FFFD.

<!-- @include: ../../../.fragments/modules/encoding/utf8/utf8.md -->

UTF-8 no tiene forma little-endian ni big-endian: sus valores codificados son secuencias de bytes definidos individualmente. Endianness solo se aplica cuando una codificación usa unidades de código de varios bytes, como UTF-16.
