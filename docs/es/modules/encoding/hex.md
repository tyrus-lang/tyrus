---
title: Hexadecimal
description: Codificación hexadecimal de bytes en minúsculas y decodificación estricta.
---

# Hexadecimal

<SignatureBlock label="Firma">hex.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Firma">hex.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Importe `hex` de `tyrus:encoding`. `encode(bytes)` produce la representación hexadecimal canónica en minúsculas del array de bytes, sin prefijo `0x`. `decode(text)` acepta solo pares completos de dígitos ASCII hexadecimales; longitud impar, prefijo `0x`, espacios y caracteres no válidos devuelven `Err(EncodingError)`. El decodificador acepta solo el alfabeto canónico en minúsculas, por lo que los dígitos en mayúsculas se rechazan en lugar de normalizarse silenciosamente.

<!-- @include: ../../../.fragments/modules/encoding/hex-usage.md -->

Cada par de dígitos hexadecimales nombra exactamente un byte, de modo que el resultado no lleva endianness: es la grafía directa en base 16 de un array de bytes.