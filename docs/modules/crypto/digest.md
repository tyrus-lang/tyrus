---
title: digest
description: Cryptographic hash functions through tyrus:crypto.
---

# digest

The `digest` namespace provides one-shot and incremental hashing.

<!-- @include: ../../.fragments/modules/crypto/digest/digest.md -->

## Functions

| Function | Signature |
|----------|-----------|
| `digest.sha256` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.sha384` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.sha512` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.md5` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.create` | `(string) => Result<DigestStream, CryptoError>` |

::: danger ATTENTION
`digest.md5` exists only for compatibility checksums; MD5 is collision-broken and must not protect passwords, signatures, certificates, or security-sensitive integrity.
:::

## DigestStream

`digest.create("sha256" | "sha384" | "sha512")` returns a `Result<DigestStream, CryptoError>` for incremental hashing:

| Method | Signature | Description |
|--------|-----------|-------------|
| `update` | `(string \| u8[]) => Result<DigestStream, CryptoError>` | Feed data, returns self |
| `digest` | `() => Result<u8[], CryptoError>` | Finalize and return hash |
| `reset` | `() => Result<DigestStream, CryptoError>` | Reset state, returns self |
| `dispose` | `() => void` | Deterministic early release |

The native handle is released automatically at the end of its lexical scope; `dispose()` permits deterministic early release and must be called at most once. Unknown algorithms are rejected with `ERR_CRYPTO_DIGEST_ALGORITHM`.

<!-- @include: ../../.fragments/modules/crypto/digest/digeststream.md -->
