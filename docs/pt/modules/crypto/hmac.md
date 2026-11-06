---
title: hmac
description: Autenticação por hash com chave através de tyrus:crypto.
---

# hmac

O namespace `hmac` fornece funções HMAC para mensagens autenticadas.

<!-- @include: ../../../.fragments/modules/crypto/hmac/hmac.md -->

## Funções

| Função | Assinatura |
|--------|-----------|
| `hmac.sha256` | `(string \| u8[], string \| u8[]) => Result<u8[], CryptoError>` |
| `hmac.sha384` | `(string \| u8[], string \| u8[]) => Result<u8[], CryptoError>` |
| `hmac.sha512` | `(string \| u8[], string \| u8[]) => Result<u8[], CryptoError>` |

Tanto `data` quanto `key` aceitam `string | u8[]`. Cópias temporárias de chave controladas pelo runtime são zeroizadas após o uso.

Use `timingSafeEqual` para verificar HMACs em tempo constante.
