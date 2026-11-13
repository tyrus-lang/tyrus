---
title: rand
description: Geração aleatória criptograficamente segura através de tyrus:crypto.
---

# rand

O namespace `rand` fornece geração de valores aleatórios criptograficamente seguros.

<!-- @include: ../../../.fragments/modules/crypto/rand/rand.md -->

## Funções

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| `rand.randomBytes` | `(u32) => Result<u8[], CryptoError>` | Bytes CSPRNG (limitado a 16 MiB por chamada) |
| `rand.randomInt` | `(i64, i64) => Result<i64, CryptoError>` | Inteiro aleatório no intervalo inclusivo; retorna `ERR_CRYPTO_INVALID_RANGE` quando `min > max` |
| `rand.randomFloat` | `() => f64` | Float aleatório em `[0, 1)` |
| `rand.uuidv4` | `() => string` | UUID aleatório |
| `rand.uuidv7` | `() => string` | UUID ordenado por tempo |
| `rand.ulid` | `() => Result<string, CryptoError>` | ULID monotônico |

`rand.ulid()` usa um gerador sincronizado global ao processo para permanecer estritamente monotônico em chamadas consecutivas.

## Erros

`CryptoError` tem os campos somente leitura `code` e `message`. Importe `CryptoError` ao citá-lo numa anotação de tipo; os campos do erro inferido ficam disponíveis quando a interface é importada. `ERR_CRYPTO_INVALID_RANGE` indica que `min` excede `max`, `ERR_CRYPTO_RANDOM_LIMIT` indica que a quantidade solicitada excede 16 MiB e `ERR_CRYPTO_RANDOM_FAILED` indica falha da fonte de aleatoriedade do sistema ou do gerador monotônico. Essas operações retornam `Err(CryptoError)` e não lançam.
