---
title: jwt
description: Firma, verificación y JWKS de JSON Web Token a través de tyrus:crypto.
---

# jwt

El namespace `jwt` proporciona firma, verificación y gestión de claves JWKS de JWT.

<!-- @include: ../../../.fragments/modules/crypto/jwt/jwt.md -->

## Funciones

| Función | Firma |
|---------|-------|
| `jwt.sign<T>` | `<T>(T, string, options?) => Result<string, CryptoError>` |
| `jwt.verify<T>` | `<T>(string, string, options?) => Result<T, CryptoError>` |
| `jwt.verifyWithJwks<T>` | `<T>(string, string, options?) => Result<T, CryptoError>` |
| `jwt.decodeUnverified<T>` | `<T>(string) => Result<T, CryptoError>` |
| `jwt.fetchJwks` | `(string, options?) => Promise<Result<string, CryptoError>>` |

### jwt.sign

Acepta un objeto tipado de claims y admite HS256/384/512 con un secreto de 32 a 65.536 bytes y RS256/384/512, ES256/384 o EdDSA con una clave privada PEM. `T` se infiere de `claims`; los campos admitidos son `string`, `boolean`, `f32`/`f64` finitos e `i32`/`i64`/`u32`/`u64`. Los fallos recuperables se devuelven como `Result<string, CryptoError>` y se pueden gestionar con `match` o propagar con `?`. Las opciones seleccionan `algorithm`, `audience`, `expiresIn`, `issuer`, `jwtId`, `keyId`, `noTimestamp` y `subject`. La firma añade `iat` salvo que se deshabilite y añade `exp` solo cuando `expiresIn` está presente. Los payloads y tokens se limitan a 1 MiB.

### jwt.verify

Acepta un secreto simétrico, una clave pública PEM o una JWK pública asimétrica codificada como JSON; los valores JWK simétricos `oct` se rechazan para impedir que secrets se introduzcan por la ruta de clave pública. El argumento de tipo explícito obligatorio hace inequívoco el layout esperado de los claims; el runtime analiza el payload una sola vez, valida cada campo declarado y rango numérico, ignora claims registrados adicionales y devuelve `Err(CryptoError)` con `ERR_CRYPTO_JWT_INVALID_CLAIMS` para campos ausentes, incompatibles o fuera de rango. La lista permitida predeterminada contiene solo HS256; las aplicaciones que usan otro algoritmo deben pasar `algorithms`, y el algoritmo del token, la familia de la clave, `alg`, `use` y `key_ops` de la JWK deben coincidir. La verificación aplica `exp` y `nbf` cuando están presentes. Las opciones también restringen `audience`, `issuer`, `subject`, `clockTolerance` y `maxAge`. La verificación y la decodificación devuelven `Result<T, CryptoError>` para fallos recuperables; gestiónelos con `match` o propáguelos con `?`. Un header JOSE `crit` se rechaza porque el runtime no implementa extensiones críticas de header.

### jwt.fetchJwks

Devuelve `Promise<Result<string, CryptoError>>`, que se cumple con el documento JWKS limitado o `Err(CryptoError)` ante fallos de política de URL, DNS, red, estado HTTP, tamaño de respuesta, documento no válido o capacidad de caché. Exige HTTPS en el puerto 443, rechaza credenciales, fragments, destinos por IP literal y no públicos, fija resultados DNS validados para impedir rebinding, deshabilita proxies y redirecciones, aplica timeouts de conexión y total, limita el cuerpo a 256 KiB y el conjunto a 64 claves públicas asimétricas de verificación con identificadores únicos, y usa una caché de diez minutos limitada a 32 entradas con coalescencia single-flight. Pase `{ forceRefresh: true }` después de una señal controlada de rotación de claves para omitir una entrada de caché aún válida.

### jwt.verifyWithJwks

Exige exactamente una clave pública asimétrica compatible que coincida con el `kid` del header.

### jwt.decodeUnverified

Analiza y comprueba los tipos de los claims sin autenticarlos. Su resultado está controlado por el atacante y nunca debe autorizar acceso, establecer identidad ni tomar una decisión de seguridad.

::: danger Advertencia de seguridad
`jwt.decodeUnverified` no autentica el token. Nunca use su resultado para decisiones de autorización o identidad.
:::
