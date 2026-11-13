---
title: Hexadecimal
description: Canonical lowercase hexadecimal byte encoding and strict decoding.
---

# Hexadecimal

<SignatureBlock label="Signature">hex.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Signature">hex.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Import `hex` from `tyrus:encoding`. `encode(bytes)` produces the canonical lowercase hexadecimal representation of the byte array, without an `0x` prefix. `decode(text)` accepts only complete pairs of ASCII hexadecimal digits; odd length, an `0x` prefix, whitespace, and invalid characters return `Err(EncodingError)`. The decoder accepts only the lowercase canonical alphabet, so uppercase digits are rejected instead of silently normalized.

<!-- @include: ../../.fragments/modules/encoding/hex-usage.md -->

Each pair of hexadecimal digits names exactly one byte, so the result carries no endianness: it is the direct base-16 spelling of a byte array.