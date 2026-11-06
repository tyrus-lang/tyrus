---
title: Encoding
description: Explicit UTF-8, UTF-16, hexadecimal, Base64, Base32, and radix conversion.
---

# Encoding

`tyrus:encoding` keeps `string` in UTF-8 and exposes explicit conversion at data boundaries. Import `utf8` for UTF-8 bytes, `utf16` for UTF-16 code units or little-endian and big-endian byte sequences, `hex` for canonical lowercase hexadecimal byte encoding, `base64` and `base64Url` for standard or URL-safe base64, `base32` for canonical uppercase base32, and `radix` for integer formatting and parsing in bases 2, 8, 10, and 16. Strict decoders and radix parsing return `Result<..., EncodingError>`, which callers inspect with `match` or propagate with `?`; encoders, `radix.format`, and the `decodeLossy`, `decodeLELossy`, and `decodeBELossy` decoders are infallible, and the lossy decoders replace malformed input with U+FFFD.

## API

- [UTF-8](./utf8): `encode`, strict `decode`, and recovery `decodeLossy`.
- [UTF-16](./utf16): code units, explicit LE/BE byte order, strict decoders, and lossy decoders.
- [Hexadecimal](./hex): canonical lowercase encoding and strict decoding.
- [Base64](./base64): standard `+/` alphabet with mandatory padding and strictly canonical decoding.
- [Base64Url](./base64url): unpadded URL-safe `-_` alphabet and strictly canonical decoding.
- [Base32](./base32): canonical uppercase encoding with padding and strictly canonical decoding.
- [Radix](./radix): integer formatting and parsing in bases 2, 8, 10, and 16.
- [EncodingError](./errors): the error payload returned by strict decoders and radix parsing.