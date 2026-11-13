---
title: rand
description: Cryptographically secure random generation through tyrus:crypto.
---

# rand

The `rand` namespace provides cryptographically secure random value generation.

<!-- @include: ../../.fragments/modules/crypto/rand/rand.md -->

## Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `rand.randomBytes` | `(u32) => Result<u8[], CryptoError>` | CSPRNG bytes (limited to 16 MiB per call) |
| `rand.randomInt` | `(i64, i64) => Result<i64, CryptoError>` | Random integer in the inclusive range, returns `ERR_CRYPTO_INVALID_RANGE` when `min > max` |
| `rand.randomFloat` | `() => f64` | Random float in `[0, 1)` |
| `rand.uuidv4` | `() => string` | Random UUID |
| `rand.uuidv7` | `() => string` | Time-ordered UUID |
| `rand.ulid` | `() => Result<string, CryptoError>` | Monotonic ULID |

`rand.ulid()` uses a process-wide synchronized generator to remain strictly monotonic for consecutive calls.

## Errors

`CryptoError` has readonly `code` and `message` fields. Import `CryptoError` when naming it in a type annotation; inferred error fields are available when the interface is imported. `ERR_CRYPTO_INVALID_RANGE` means `min` exceeds `max`, `ERR_CRYPTO_RANDOM_LIMIT` means the requested byte count exceeds 16 MiB, and `ERR_CRYPTO_RANDOM_FAILED` means the system random source or monotonic generator failed. These operations return `Err(CryptoError)` and do not throw.
