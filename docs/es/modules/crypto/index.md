---
title: Crypto
description: Primitivas criptográficas nativas con límites explícitos y cifrado autenticado.
---

# Crypto

Importa utilidades criptográficas desde `tyrus:crypto`. Los fallos recuperables de las APIs de crypto devuelven valores `Result` tipados; las operaciones asíncronas resuelven `Promise<Result<T, CryptoError>>` en lugar de rechazar por esos fallos. Las funciones `throws` definidas por el usuario siguen disponibles de forma independiente. Un throw del usuario no capturado se escribe en stderr y termina el proceso con estado distinto de cero.

## Bytes y límites

Use `tyrus:encoding` para conversión UTF-8 y UTF-16. Las entradas generales de crypto están limitadas a 64 MiB, y `rand.randomBytes` está limitado a 16 MiB por llamada.

<!-- @include: ../../../.fragments/modules/crypto/basic-usage.md -->

## Digest y HMAC

`digest.sha256`, `digest.sha384` y `digest.sha512` aceptan `string | u8[]` y devuelven `Result<u8[], CryptoError>`; `digest.md5` tiene el mismo retorno y existe solo para checksums de compatibilidad. MD5 tiene colisiones conocidas y no debe proteger contraseñas, firmas, certificados ni integridad sensible a la seguridad.

`hmac.sha256`, `hmac.sha384` y `hmac.sha512` aceptan `string | u8[]` para datos y clave y devuelven `Result<u8[], CryptoError>`. Las copias temporales de claves que realiza el runtime se ponen a cero después de usarlas.

`digest.create("sha256" | "sha384" | "sha512")` devuelve `Result<DigestStream, CryptoError>`. Los métodos `update(data)`, `digest()` y `reset()` también devuelven valores `Result`. Trátalos con `match` o propágalos con `?`. El handle nativo se libera automáticamente al final de su ámbito léxico; `dispose()` permite la liberación anticipada determinista y debe llamarse como máximo una vez. Los algoritmos desconocidos devuelven `Err` con `ERR_CRYPTO_DIGEST_ALGORITHM`.

## Comparación en tiempo constante

`timingSafeEqual(a: string | u8[], b: string | u8[]): boolean` compara datos del mismo tamaño en tiempo constante. Tamaños diferentes devuelven `false` inmediatamente; la función no oculta el tamaño de los datos.

## Cifrado autenticado

`cipher.aes.encrypt(data: string | u8[], key: u8[]): Result<u8[], CryptoError>` crea un envelope AES-256-GCM versionado; los métodos de descifrado también devuelven `Result` con `CryptoError` para fallos recuperables. `decryptText` autentica antes de decodificar UTF-8, mientras que `decryptBytes` autentica antes de devolver bytes. La clave debe contener exactamente 32 bytes.

`encryptWithAad`, `decryptTextWithAad` y `decryptBytesWithAad` vinculan datos autenticados adicionales al envelope. El descifrado debe recibir AAD idéntico byte por byte. El envelope autentica magic, versión, identificador del algoritmo, nonce, ciphertext, tag y el AAD proporcionado.

<!-- @include: ../../../.fragments/modules/crypto/cipher-usage.md -->

## Hashing de contraseñas

`argon2.hash(password: string | u8[], options?): Promise<Result<string, CryptoError>>` usa Argon2id con salt aleatorio de 16 bytes, salida de 32 bytes y el perfil predeterminado `m=65536`, `t=3`, `p=4`. Las opciones pueden definir `type: "argon2id"`, `memoryCost` de 19.456 a 262.144 KiB, `timeCost` de 2 a 10, `parallelism` de 1 a 16 y un `secret` binario no vacío de hasta 1.024 bytes. Las contraseñas textuales y binarias se limitan a 1.024 bytes. `argon2.verify(hash, password: string | u8[], options?): Promise<Result<boolean, CryptoError>>` devuelve `Err(CryptoError)` para cadenas PHC malformadas, parámetros no válidos, falta del pepper requerido, concurrencia agotada y fallos del worker; limita las cadenas PHC a 4 KiB y valida los parámetros no confiables antes de asignar trabajo. Un hash con pepper contiene un marcador autenticado no secreto; omitir `secret` al verificarlo devuelve `ERR_CRYPTO_ARGON2_PEPPER_REQUIRED`, mientras que proporcionar un secret incorrecto devuelve `Ok(false)`. Almacene y rote el pepper por separado de los hashes de contraseña. `argon2.needsRehash(hash: string): Result<boolean, CryptoError>` indica si una cadena PHC válida difiere del perfil predeterminado; las cadenas malformadas o demasiado grandes devuelven `ERR_CRYPTO_ARGON2_INVALID_HASH`. 

Como máximo cuatro operaciones Argon2 se ejecutan simultáneamente. Las llamadas adicionales se rechazan con `ERR_CRYPTO_ARGON2_BUSY` en vez de formar una cola ilimitada de CPU o memoria.

El perfil predeterminado midió 127,93–145,24 ms en una baseline Criterion optimizada sobre un Intel Core i5-10400F de seis núcleos. Esta es una referencia reproducible para ese host, no una promesa universal de latencia; mida el hardware de producción y reduzca la concurrencia cuando lo exija su presupuesto de memoria o latencia.

## JWT

`jwt.sign<T>(claims: T, key: string, options?): Result<string, CryptoError>` acepta un objeto tipado de claims y admite HS256/384/512 con un secreto de 32 a 65.536 bytes y RS256/384/512, ES256/384 o EdDSA con una clave privada PEM. `T` se infiere de `claims`; los campos admitidos son `string`, `boolean`, `f32`/`f64` finitos e `i32`/`i64`/`u32`/`u64`. Las opciones seleccionan `algorithm`, `audience`, `expiresIn`, `issuer`, `jwtId`, `keyId`, `noTimestamp` y `subject`. La firma añade `iat` salvo que se deshabilite y añade `exp` solo cuando `expiresIn` está presente. Los payloads y tokens se limitan a 1 MiB.

`jwt.verify<T>(token, key, options?): Result<T, CryptoError>` acepta un secreto simétrico, una clave pública PEM o una JWK pública asimétrica codificada como JSON; los valores JWK simétricos `oct` se rechazan para impedir que secrets se introduzcan por la ruta de clave pública. El argumento de tipo explícito obligatorio hace inequívoco el layout esperado de los claims; el runtime analiza el payload una sola vez, valida cada campo declarado y rango numérico, ignora claims registrados adicionales y devuelve `Err(CryptoError)` con `ERR_CRYPTO_JWT_INVALID_CLAIMS` para campos ausentes, incompatibles o fuera de rango. La lista permitida predeterminada contiene solo HS256; las aplicaciones que usan otro algoritmo deben pasar `algorithms`, y el algoritmo del token, la familia de la clave, `alg`, `use` y `key_ops` de la JWK deben coincidir. La verificación aplica `exp` y `nbf` cuando están presentes. Las opciones también restringen `audience`, `issuer`, `subject`, `clockTolerance` y `maxAge`; audience, issuer o subject configurado exige que el claim correspondiente esté presente y coincida, y `clockTolerance` se aplica a los claims temporales registrados. Un header JOSE `crit` se rechaza porque el runtime no implementa extensiones críticas de header.

`jwt.fetchJwks(url, options?): Promise<Result<string, CryptoError>>` obtiene un documento JWKS limitado mediante HTTPS en el puerto 443. Se cumple con `Err(CryptoError)` ante fallos recuperables de URL, DNS, red, HTTP, tamaño, documento y caché; no permite credenciales, fragments, destinos por IP literal y no públicos, fija resultados DNS validados para impedir rebinding, deshabilita proxies y redirecciones, aplica timeouts de conexión y total, limita el cuerpo a 256 KiB y el conjunto a 64 claves públicas asimétricas de verificación con identificadores únicos, y usa una caché de diez minutos limitada a 32 entradas con coalescencia single-flight. Pase `{ forceRefresh: true }` después de una señal controlada de rotación de claves para omitir una entrada de caché aún válida. `jwt.verifyWithJwks<T>(token, jwksJson, options?): Result<T, CryptoError>` exige exactamente una clave pública asimétrica compatible que coincida con el `kid` del header.

`jwt.decodeUnverified<T>(token): Result<T, CryptoError>` analiza y comprueba los tipos de los claims sin autenticarlos. Gestione el resultado con `match` o propáguelo con `?`; el valor de éxito está controlado por el atacante y nunca debe autorizar acceso, establecer identidad ni tomar una decisión de seguridad.

<!-- @include: ../../../.fragments/modules/crypto/async-usage.md -->

## Aleatoriedad segura

`rand.randomBytes(length)`, `rand.randomInt(min, max)` y `rand.randomFloat()` usan aleatoriedad criptográficamente segura. `randomBytes`, `randomInt` y `ulid` devuelven `Result<..., CryptoError>` en fallos recuperables; manéjelos con `match` o propáguelos con `?`. `randomInt` es inclusivo y devuelve `ERR_CRYPTO_INVALID_RANGE` cuando `min > max`. `rand.uuidv4()` crea UUIDs aleatorios, `rand.uuidv7()` crea UUIDs ordenados por tiempo y `rand.ulid()` usa un generador sincronizado global al proceso para mantenerse estrictamente monotónico en llamadas consecutivas.
