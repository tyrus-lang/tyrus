---
title: Radix
description: Integer formatting and parsing in bases 2, 8, 10, and 16.
---

# Radix

<SignatureBlock label="Signature">radix.format(value: i64, base: 2 | 8 | 10 | 16): string</SignatureBlock>

<SignatureBlock label="Signature">radix.parseI64(text: string, base: 2 | 8 | 10 | 16): Result&lt;i64, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Signature">radix.parseU64(text: string, base: 2 | 8 | 10 | 16): Result&lt;u64, EncodingError&gt;</SignatureBlock>

Import `radix` from `tyrus:encoding`. `format(value, base)` returns the value in the given base using lowercase digits, with a leading `-` for negative values, and unsigned magnitude. The only supported bases are 2, 8, 10, and 16, and `base` is typed `2 | 8 | 10 | 16`, so any other base is a compile-time type error and `format` never throws. `parseI64` and `parseU64` reverse the formatting: they accept only lowercase digits, reject any prefix, and never round. `parseI64` allows one leading `-`, rejects `+`, and overflows with `ERR_ENCODING_INPUT_TOO_LARGE`; `parseU64` rejects any sign.

<!-- @include: ../../.fragments/modules/encoding/radix-usage.md -->

`format` and `parse` are exact inverses for every representable value: no sign is invented, no digit is folded, and each base has exactly one canonical spelling that round-trips. Uppercase digits such as `FF` are rejected, and a base other than 2, 8, 10, or 16 does not type-check.