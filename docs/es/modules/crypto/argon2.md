---
title: argon2
description: Hashing de contraseñas Argon2id a través de tyrus:crypto.
---

# argon2

El namespace `argon2` proporciona hashing de contraseñas Argon2id con parámetros configurables.

<!-- @include: ../../../.fragments/modules/crypto/async-usage.md -->

## Funciones

| Función | Firma |
|---------|-------|
| `argon2.hash` | `(string \| u8[], options?) => Promise<Result<string, CryptoError>>` |
| `argon2.verify` | `(string, string \| u8[], options?) => Promise<Result<boolean, CryptoError>>` |
| `argon2.needsRehash` | `(string) => Result<boolean, CryptoError>` |

`argon2.hash` cumple con `Ok(hash)` si tiene éxito y `Err(CryptoError)` ante una entrada u opciones no válidas, falta de cupos de concurrencia, fallos del generador aleatorio o del worker. Usa Argon2id con salt aleatorio de 16 bytes, salida de 32 bytes y el perfil predeterminado `m=65536`, `t=3`, `p=4`. Las opciones pueden definir `type: "argon2id"`, `memoryCost` de 19.456 a 262.144 KiB, `timeCost` de 2 a 10, `parallelism` de 1 a 16 y un `secret` binario no vacío de hasta 1.024 bytes. Las contraseñas textuales y binarias se limitan a 1.024 bytes.

`argon2.verify` cumple con `Ok(matches)` para un hash válido y `Err(CryptoError)` para cadenas PHC malformadas, parámetros no válidos, falta del pepper requerido, falta de cupos de concurrencia o fallo del worker. Limita las cadenas PHC a 4 KiB y valida los parámetros no confiables antes de asignar trabajo. Un hash con pepper contiene un marcador autenticado no secreto; omitir `secret` devuelve `ERR_CRYPTO_ARGON2_PEPPER_REQUIRED`, mientras que proporcionar un secret incorrecto devuelve `Ok(false)`. Almacene y rote el pepper por separado de los hashes de contraseña.

`argon2.needsRehash` devuelve `Ok(true)` cuando la cadena PHC es válida, pero su algoritmo, versión, parámetros, longitud del salt o longitud de salida difieren del perfil predeterminado. Las cadenas PHC no válidas o demasiado grandes devuelven `Err(CryptoError)` con el código `ERR_CRYPTO_ARGON2_INVALID_HASH`.

::: warning Límite de concurrencia
Como máximo cuatro operaciones Argon2 se ejecutan simultáneamente. Las llamadas adicionales se cumplen con `Err(CryptoError)` y el código `ERR_CRYPTO_ARGON2_BUSY`.
:::
