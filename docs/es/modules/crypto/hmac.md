---
title: hmac
description: Autenticación por hash con clave a través de tyrus:crypto.
---

# hmac

El namespace `hmac` proporciona funciones HMAC para mensajes autenticados.

<!-- @include: ../../../.fragments/modules/crypto/hmac/hmac.md -->

## Funciones

| Función | Firma |
|---------|-------|
| `hmac.sha256` | `(string \| u8[], string \| u8[]) => Result<u8[], CryptoError>` |
| `hmac.sha384` | `(string \| u8[], string \| u8[]) => Result<u8[], CryptoError>` |
| `hmac.sha512` | `(string \| u8[], string \| u8[]) => Result<u8[], CryptoError>` |

Tanto `data` como `key` aceptan `string | u8[]`. Las copias temporales de claves controladas por el runtime se ponen a cero después de usarlas.

Use `timingSafeEqual` para verificar HMACs en tiempo constante.
