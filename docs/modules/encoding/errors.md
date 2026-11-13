---
title: EncodingError
description: Errors from strict text decoders.
---

# EncodingError

<SignatureBlock label="Signature">interface EncodingError { readonly code: string; readonly message: string; }</SignatureBlock>

`EncodingError` is the error payload returned by strict decoders and radix parsing in `Result<T, EncodingError>`. Handle it with `match` or propagate it with `?`.

Strict decoders in `tyrus:encoding` return `Err(EncodingError)` when their input is not well-formed. Its `code` is stable for programmatic handling and its `message` describes the failure. The strict codecs and radix parsing share the stable codes `ERR_ENCODING_INVALID_UTF8`, `ERR_ENCODING_INVALID_UTF16`, `ERR_ENCODING_INVALID_HEX`, `ERR_ENCODING_INVALID_BASE64`, `ERR_ENCODING_INVALID_BASE32`, `ERR_ENCODING_INVALID_RADIX`, and `ERR_ENCODING_INPUT_TOO_LARGE` (for radix overflow).

<!-- @include: ../../.fragments/modules/encoding/errors/encodingerror.md -->

Use a lossy decoder when replacement with U+FFFD is the desired result, rather than catching an error.

Import `EncodingError` by name when referring to it in a type annotation. Strict decoders and radix parsing return `Result` values, so inspect each outcome with `match` or propagate it with `?`. Encoders, lossy decoders, and `radix.format` are infallible.
