---
title: Base32
description: Canonical uppercase base32 byte encoding with padded, strict decoding.
---

# Base32

<SignatureBlock label="Signature">base32.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Signature">base32.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Import `base32` from `tyrus:encoding`. `encode(bytes)` produces the canonical uppercase RFC 4648 base32 representation, using the `A`-`Z` and `2`-`7` alphabet with mandatory `=` padding so every non-empty result has a length divisible by 8. `decode(text)` reverses the encoding strictly: only the canonical form is accepted.

<!-- @include: ../../.fragments/modules/encoding/base32-usage.md -->

A strict decoder rejects anything the canonical encoder could not have produced with `ERR_ENCODING_INVALID_BASE32`: lowercase letters, characters outside the `A`-`Z`/`2`-`7` alphabet, a length not divisible by 8, `=` in the middle, padding counts that do not match the byte remainder, and trailing bits that were not zero in the source. Whitespace is never ignored. Variants such as Crockford Base32 or unpadded base32 are not part of this module.