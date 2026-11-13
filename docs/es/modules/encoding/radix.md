---
title: Radix
description: Formato y análisis de enteros en las bases 2, 8, 10 y 16.
---

# Radix

<SignatureBlock label="Firma">radix.format(value: i64, base: 2 | 8 | 10 | 16): string</SignatureBlock>

<SignatureBlock label="Firma">radix.parseI64(text: string, base: 2 | 8 | 10 | 16): Result&lt;i64, EncodingError&gt;</SignatureBlock>

<SignatureBlock label="Firma">radix.parseU64(text: string, base: 2 | 8 | 10 | 16): Result&lt;u64, EncodingError&gt;</SignatureBlock>

Importe `radix` de `tyrus:encoding`. `format(value, base)` devuelve el valor en la base indicada usando dígitos en minúsculas, con `-` inicial para valores negativos y magnitud sin signo. Las únicas bases admitidas son 2, 8, 10 y 16, y `base` tiene el tipo `2 | 8 | 10 | 16`, así que cualquier otra base es un error de tipo en compilación y `format` nunca lanza. `parseI64` y `parseU64` invierten el formato: aceptan solo dígitos en minúsculas, rechazan cualquier prefijo y nunca redondean. `parseI64` admite un único `-` inicial, rechaza `+` y se desborda con `ERR_ENCODING_INPUT_TOO_LARGE`; `parseU64` rechaza cualquier signo.

<!-- @include: ../../../.fragments/modules/encoding/radix-usage.md -->

`format` y `parse` son inversos exactos para todo valor representable: no se inventa ningún signo, no se normaliza ningún dígito y cada base tiene exactamente una grafía canónica que hace round-trip. Los dígitos en mayúsculas como `FF` se rechazan, y una base distinta de 2, 8, 10 o 16 no pasa la comprobación de tipos.