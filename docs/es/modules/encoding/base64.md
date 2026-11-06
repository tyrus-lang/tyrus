---
title: Base64
description: Codificación de bytes en base64 estándar con decodificación canónica y relleno.
---

# Base64

<SignatureBlock label="Firma">base64.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Firma">base64.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Importe `base64` de `tyrus:encoding`. `encode(bytes)` produce la representación base64 estándar RFC 4648 del array de bytes, usando el alfabeto `+/` y el relleno `=` obligatorio, de modo que todo resultado no vacío tenga una longitud divisible por 4. `decode(text)` invierte la codificación estrictamente: solo se acepta la forma canónica.

<!-- @include: ../../../.fragments/modules/encoding/base64-usage.md -->

Un decodificador estricto rechaza todo lo que el codificador canónico no habría podido producir con `ERR_ENCODING_INVALID_BASE64`: caracteres fuera del alfabeto `+/`, relleno en el medio, longitud no divisible por 4, `=` cuando el resto espera uno (para 2 bytes) o dos (para 1 byte), y bits finales que no eran cero en el origen. Los espacios nunca se ignoran.