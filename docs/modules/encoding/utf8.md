---
title: UTF-8
description: UTF-8 byte conversion.
---

# UTF-8

<SignatureBlock label="Signature">utf8.encode(text: string): u8[]</SignatureBlock>

<SignatureBlock label="Signature">utf8.decode(bytes: u8[]): Result&lt;string, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Signature">utf8.decodeLossy(bytes: u8[]): string</SignatureBlock>

Import `utf8` from `tyrus:encoding`. `encode(text)` returns the UTF-8 bytes of a string. `decode(bytes)` accepts only well-formed UTF-8 and returns `Err(EncodingError)` otherwise. `decodeLossy(bytes)` instead replaces each malformed sequence with U+FFFD.

<!-- @include: ../../.fragments/modules/encoding/utf8/utf8.md -->

UTF-8 has no little-endian or big-endian form: its encoded values are sequences of individually defined bytes. Endianness applies only when a character encoding uses multi-byte code units, such as UTF-16.
