---
title: digest
description: Funções de hash criptográfico através de tyrus:crypto.
---

# digest

O namespace `digest` fornece hashing de uso único e incremental.

<!-- @include: ../../../.fragments/modules/crypto/digest/digest.md -->

## Funções

| Função | Assinatura |
|--------|-----------|
| `digest.sha256` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.sha384` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.sha512` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.md5` | `(string \| u8[]) => Result<u8[], CryptoError>` |
| `digest.create` | `(string) => Result<DigestStream, CryptoError>` |

::: danger ATENÇÃO
`digest.md5` existe apenas para checksums de compatibilidade; MD5 possui colisões conhecidas e não deve proteger senhas, assinaturas, certificados ou integridade sensível à segurança.
:::

## DigestStream

`digest.create("sha256" | "sha384" | "sha512")` retorna um `Result<DigestStream, CryptoError>` para hashing incremental:

| Método | Assinatura | Descrição |
|--------|-----------|-----------|
| `update` | `(string \| u8[]) => Result<DigestStream, CryptoError>` | Alimenta dados, retorna self |
| `digest` | `() => Result<u8[], CryptoError>` | Finaliza e retorna hash |
| `reset` | `() => Result<DigestStream, CryptoError>` | Reseta estado, retorna self |
| `dispose` | `() => void` | Liberação antecipada determinística |

O handle nativo é liberado automaticamente ao final do escopo léxico; `dispose()` permite liberação antecipada determinística e deve ser chamado no máximo uma vez. Algoritmos desconhecidos são rejeitados com `ERR_CRYPTO_DIGEST_ALGORITHM`.

<!-- @include: ../../../.fragments/modules/crypto/digest/digeststream.md -->
