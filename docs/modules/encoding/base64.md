---
title: Base64
description: Standard base64 byte encoding with padded, canonical decoding.
---

# Base64

<SignatureBlock label="Signature">base64.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Signature">base64.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Import `base64` from `tyrus:encoding`. `encode(bytes)` produces the standard RFC 4648 base64 representation of the byte array, using the `+/` alphabet and mandatory `=` padding so every non-empty result has a length divisible by 4. `decode(text)` reverses the encoding strictly: only the canonical form is accepted.

<!-- @include: ../../.fragments/modules/encoding/base64-usage.md -->

A strict decoder rejects anything the canonical encoder could not have produced with `ERR_ENCODING_INVALID_BASE64`: characters outside the `+/` alphabet, padding in the middle, a length not divisible by 4, `=` where the remainder expects one for 2 bytes or two for 1 byte, and trailing bits that were not zero in the source. Whitespace is never ignored.