---
title: hmac
description: Keyed-hash message authentication through tyrus:crypto.
---

# hmac

The `hmac` namespace provides HMAC functions for authenticated messaging.

<!-- @include: ../../.fragments/modules/crypto/hmac/hmac.md -->

## Functions

| Function | Signature |
|----------|-----------|
| `hmac.sha256` | `(string \| u8[], string \| u8[]) => Result<u8[], CryptoError>` |
| `hmac.sha384` | `(string \| u8[], string \| u8[]) => Result<u8[], CryptoError>` |
| `hmac.sha512` | `(string \| u8[], string \| u8[]) => Result<u8[], CryptoError>` |

Both `data` and `key` accept `string | u8[]`. Temporary key copies controlled by the runtime are zeroized after use.

Use `timingSafeEqual` to verify HMACs in constant time.
