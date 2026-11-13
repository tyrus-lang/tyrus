---
title: Base64Url
description: Codificación de bytes en base64 segura para URLs, sin relleno, y decodificación estricta.
---

# Base64Url

<SignatureBlock label="Firma">base64Url.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Firma">base64Url.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Importe `base64Url` de `tyrus:encoding`. `encode(bytes)` produce la representación base64 RFC 4648 segura para URLs, sustituyendo `+` por `-` y `/` por `_` y omitiendo el relleno `=`. El alfabeto no contiene caracteres que requieran percent-encoding en URLs o dentro de strings JSON, y la forma sin relleno sigue siendo la más corta. `decode(text)` invierte la codificación estrictamente: el relleno y los caracteres del alfabeto estándar se rechazan con `ERR_ENCODING_INVALID_BASE64`.

<!-- @include: ../../../.fragments/modules/encoding/base64url-usage.md -->

Como se omite el relleno, la longitud de una entrada válida nunca es 1 módulo 4; cualquier otra longitud se rechaza. A diferencia del alfabeto estándar, la variante segura para URLs no es autodelimitante, por lo que `encode` y `decode` son inversos exactos entre sí y nunca aceptan entrada con `=`.