---
title: argon2
description: Argon2id password hashing through tyrus:crypto.
---

# argon2

The `argon2` namespace provides Argon2id password hashing with configurable parameters.

<!-- @include: ../../.fragments/modules/crypto/async-usage.md -->

## Functions

| Function | Signature |
|----------|-----------|
| `argon2.hash` | `(string \| u8[], options?) => Promise<Result<string, CryptoError>>` |
| `argon2.verify` | `(string, string \| u8[], options?) => Promise<Result<boolean, CryptoError>>` |
| `argon2.needsRehash` | `(string) => Result<boolean, CryptoError>` |

`argon2.hash` fulfills with `Ok(hash)` on success and `Err(CryptoError)` on invalid input, invalid options, exhausted concurrency, random source failure, or worker failure. It uses Argon2id with a random 16-byte salt, a 32-byte output, and the default profile `m=65536`, `t=3`, `p=4`. Options may set `type: "argon2id"`, `memoryCost` from 19,456 to 262,144 KiB, `timeCost` from 2 to 10, `parallelism` from 1 to 16, and a non-empty binary `secret` up to 1,024 bytes. Text and binary passwords are limited to 1,024 bytes.

`argon2.verify` fulfills with `Ok(matches)` for a valid hash and `Err(CryptoError)` for malformed PHC strings, invalid parameters, missing required pepper, exhausted concurrency, or worker failure. PHC strings are limited to 4 KiB and untrusted parameters are validated before allocating work. A peppered hash contains a non-secret authenticated marker; omitting `secret` returns `ERR_CRYPTO_ARGON2_PEPPER_REQUIRED`, while a supplied but incorrect secret returns `Ok(false)`. Store and rotate pepper separately from password hashes.

`argon2.needsRehash` returns `Ok(true)` when the PHC string is valid but its algorithm, version, parameters, salt length, or output length differs from the default profile. Invalid or oversized PHC strings return `Err(CryptoError)` with code `ERR_CRYPTO_ARGON2_INVALID_HASH`.

::: warning Concurrency limit
At most four Argon2 operations run concurrently. Additional calls fulfill with `Err(CryptoError)` using code `ERR_CRYPTO_ARGON2_BUSY`.
:::
