---
title: UTF-16
description: Conversión de unidades de código y orden de bytes UTF-16.
---

# UTF-16

<SignatureBlock label="Firma">utf16.encode(text: string): u16[]</SignatureBlock>

<SignatureBlock label="Firma">utf16.encodeLE(text: string): u8[]</SignatureBlock>

<SignatureBlock label="Firma">utf16.encodeBE(text: string): u8[]</SignatureBlock>

<SignatureBlock label="Firma">utf16.decode(units: u16[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Firma">utf16.decodeLossy(units: u16[]): string</SignatureBlock>

<SignatureBlock label="Firma">utf16.decodeLE(bytes: u8[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Firma">utf16.decodeLELossy(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Firma">utf16.decodeBE(bytes: u8[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Firma">utf16.decodeBELossy(bytes: u8[]): string</SignatureBlock>

Importe `utf16` de `tyrus:encoding`. `encode(text)` devuelve unidades `u16` independientes del host. `encodeLE(text)` y `encodeBE(text)` devuelven arrays de bytes en el orden elegido explícitamente. `decode`, `decodeLE` y `decodeBE` son estrictos; los métodos `Lossy` correspondientes sustituyen secuencias surrogate no válidas por U+FFFD.

<!-- @include: ../../../.fragments/modules/encoding/utf16/utf16.md -->

Los decodificadores orientados a bytes requieren una cantidad par de bytes. Las variantes estrictas devuelven `Err(EncodingError)` para una longitud impar o una secuencia surrogate no válida; las variantes lossy reparan solo secuencias surrogate no válidas.
