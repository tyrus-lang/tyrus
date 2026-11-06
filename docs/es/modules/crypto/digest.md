---
title: digest
description: Funciones de hash criptográfico a través de tyrus:crypto.
---

# digest

El namespace `digest` proporciona hashing de uso único e incremental.

<!-- @include: ../../../.fragments/modules/crypto/digest/digest.md -->

## Funciones

| Función | Firma |
|---------|-------|
| `digest.sha256` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.sha384` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.sha512` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.md5` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.create` | `(string) => Result<DigestStream, CryptoError>` |

::: danger ATENCIÓN
`digest.md5` existe solo para checksums de compatibilidad; MD5 tiene colisiones conocidas y no debe proteger contraseñas, firmas, certificados ni integridad sensible a la seguridad.
:::

## DigestStream

`digest.create("sha256" | "sha384" | "sha512")` devuelve un `Result<DigestStream, CryptoError>` para hashing incremental:

| Método | Firma | Descripción |
|--------|-------|-------------|
| `update` | `(string \| u8[]) => Result<DigestStream, CryptoError>` | Alimenta datos, devuelve self |
| `digest` | `() => Result<u8[], CryptoError>` | Finaliza y devuelve hash |
| `reset` | `() => Result<DigestStream, CryptoError>` | Resetea estado, devuelve self |
| `dispose` | `() => void` | Liberación anticipada determinista |

El handle nativo se libera automáticamente al final de su ámbito léxico; `dispose()` permite la liberación anticipada determinista y debe llamarse como máximo una vez. Los algoritmos desconocidos se rechazan con `ERR_CRYPTO_DIGEST_ALGORITHM`.

<!-- @include: ../../../.fragments/modules/crypto/digest/digeststream.md -->
