---
title: Base32
description: Codificación de bytes en base32 canónica en mayúsculas, con relleno y decodificación estricta.
---

# Base32

<SignatureBlock label="Firma">base32.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Firma">base32.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Importe `base32` de `tyrus:encoding`. `encode(bytes)` produce la representación RFC 4648 base32 canónica en mayúsculas, usando el alfabeto `A`-`Z` y `2`-`7` con relleno `=` obligatorio, de modo que todo resultado no vacío tenga una longitud divisible por 8. `decode(text)` invierte la codificación estrictamente: solo se acepta la forma canónica.

<!-- @include: ../../../.fragments/modules/encoding/base32-usage.md -->

Un decodificador estricto rechaza todo lo que el codificador canónico no habría podido producir con `ERR_ENCODING_INVALID_BASE32`: letras minúsculas, caracteres fuera del alfabeto `A`-`Z`/`2`-`7`, longitud no divisible por 8, `=` en el medio, recuento de relleno que no coincide con el resto de bytes, y bits finales que no eran cero en el origen. Los espacios nunca se ignoran. Las variantes como Crockford Base32 o base32 sin relleno no forman parte de este módulo.