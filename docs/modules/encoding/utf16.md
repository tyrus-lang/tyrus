---
title: UTF-16
description: UTF-16 code-unit and byte-order conversion.
---

# UTF-16

<SignatureBlock label="Signature">utf16.encode(text: string): u16[]</SignatureBlock>

<SignatureBlock label="Signature">utf16.encodeLE(text: string): u8[]</SignatureBlock>

<SignatureBlock label="Signature">utf16.encodeBE(text: string): u8[]</SignatureBlock>

<SignatureBlock label="Signature">utf16.decode(units: u16[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Signature">utf16.decodeLossy(units: u16[]): string</SignatureBlock>

<SignatureBlock label="Signature">utf16.decodeLE(bytes: u8[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Signature">utf16.decodeLELossy(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Signature">utf16.decodeBE(bytes: u8[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Signature">utf16.decodeBELossy(bytes: u8[]): string</SignatureBlock>

Import `utf16` from `tyrus:encoding`. `encode(text)` returns host-independent `u16` code units. `encodeLE(text)` and `encodeBE(text)` return byte arrays in the explicitly selected order. `decode`, `decodeLE`, and `decodeBE` are strict; the matching `Lossy` methods replace malformed surrogate sequences with U+FFFD.

<!-- @include: ../../.fragments/modules/encoding/utf16/utf16.md -->

Byte-oriented decoders require an even number of bytes. The strict variants return `Err(EncodingError)` for an odd byte length or malformed surrogate sequence; the lossy variants repair malformed surrogate sequences only.
