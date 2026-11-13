---
title: Base64Url
description: Unpadded URL-safe base64 byte encoding and strict decoding.
---

# Base64Url

<SignatureBlock label="Signature">base64Url.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Signature">base64Url.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Import `base64Url` from `tyrus:encoding`. `encode(bytes)` produces the URL-safe RFC 4648 base64 representation, substituting `-` for `+` and `_` for `/` and omitting the `=` padding. The alphabet contains no characters that need percent-encoding in URLs or within JSON strings, and the unpadded form stays the shortest one. `decode(text)` reverses the encoding strictly: padding and standard-alphabet characters are rejected with `ERR_ENCODING_INVALID_BASE64`.

<!-- @include: ../../.fragments/modules/encoding/base64url-usage.md -->

Because padding is omitted, the length of a valid input is never 1 modulo 4; anything else is rejected. Unlike the standard alphabet, the URL-safe variant is not self-delimiting, so `encode` and `decode` are exact inverses of each other and never match input with `=`.