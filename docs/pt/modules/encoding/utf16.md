---
title: UTF-16
description: Conversão de unidades de código e ordem de bytes UTF-16.
---

# UTF-16

<SignatureBlock label="Assinatura">utf16.encode(text: string): u16[]</SignatureBlock>

<SignatureBlock label="Assinatura">utf16.encodeLE(text: string): u8[]</SignatureBlock>

<SignatureBlock label="Assinatura">utf16.encodeBE(text: string): u8[]</SignatureBlock>

<SignatureBlock label="Assinatura">utf16.decode(units: u16[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Assinatura">utf16.decodeLossy(units: u16[]): string</SignatureBlock>

<SignatureBlock label="Assinatura">utf16.decodeLE(bytes: u8[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Assinatura">utf16.decodeLELossy(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Assinatura">utf16.decodeBE(bytes: u8[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Assinatura">utf16.decodeBELossy(bytes: u8[]): string</SignatureBlock>

Importe `utf16` de `tyrus:encoding`. `encode(text)` retorna unidades `u16` independentes do host. `encodeLE(text)` e `encodeBE(text)` retornam arrays de bytes na ordem escolhida explicitamente. `decode`, `decodeLE` e `decodeBE` são estritos; os métodos `Lossy` correspondentes substituem sequências surrogate inválidas por U+FFFD.

<!-- @include: ../../../.fragments/modules/encoding/utf16/utf16.md -->

Decoders orientados a bytes exigem uma quantidade par de bytes. As variantes estritas retornam `Err(EncodingError)` para comprimento ímpar ou sequência surrogate inválida; as variantes lossy reparam apenas sequências surrogate inválidas.
