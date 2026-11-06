---
title: rand
description: Generación aleatoria criptográficamente segura a través de tyrus:crypto.
---

# rand

El namespace `rand` proporciona generación de valores aleatorios criptográficamente seguros.

<!-- @include: ../../../.fragments/modules/crypto/rand/rand.md -->

## Funciones

| Función | Firma | Descripción |
|---------|-------|-------------|
| `rand.randomBytes` | `(u32) => Result<u8[], CryptoError>` | Bytes CSPRNG (limitado a 16 MiB por llamada) |
| `rand.randomInt` | `(i64, i64) => Result<i64, CryptoError>` | Entero aleatorio en el intervalo inclusivo; devuelve `ERR_CRYPTO_INVALID_RANGE` cuando `min > max` |
| `rand.randomFloat` | `() => f64` | Float aleatorio en `[0, 1)` |
| `rand.uuidv4` | `() => string` | UUID aleatorio |
| `rand.uuidv7` | `() => string` | UUID ordenado por tiempo |
| `rand.ulid` | `() => Result<string, CryptoError>` | ULID monotónico |

`rand.ulid()` usa un generador sincronizado global al proceso para mantenerse estrictamente monotónico en llamadas consecutivas.

## Errores

`CryptoError` tiene los campos de solo lectura `code` y `message`. Importe `CryptoError` cuando lo mencione en una anotación de tipo; los campos del error inferido están disponibles cuando se importa la interfaz. `ERR_CRYPTO_INVALID_RANGE` indica que `min` supera `max`, `ERR_CRYPTO_RANDOM_LIMIT` indica que la cantidad solicitada supera 16 MiB y `ERR_CRYPTO_RANDOM_FAILED` indica un fallo de la fuente aleatoria del sistema o del generador monotónico. Estas operaciones devuelven `Err(CryptoError)` y no lanzan.
