---
title: Encoding
description: Conversión explícita de UTF-8, UTF-16, hexadecimal, Base64, Base32 y radix.
---

# Encoding

`tyrus:encoding` mantiene `string` en UTF-8 y expone conversión explícita en los límites de datos. Importe `utf8` para bytes UTF-8, `utf16` para unidades UTF-16 o secuencias de bytes little-endian y big-endian, `hex` para codificación hexadecimal canónica en minúsculas, `base64` y `base64Url` para base64 estándar o segura para URLs, `base32` para base32 canónica en mayúsculas, y `radix` para formato y análisis de enteros en las bases 2, 8, 10 y 16. Los decodificadores estrictos y el análisis radix devuelven `Result<..., EncodingError>`, que se puede manejar con `match` o propagar con `?`; los codificadores, `radix.format` y los decodificadores `decodeLossy`, `decodeLELossy` y `decodeBELossy` son infalibles, y los decodificadores lossy sustituyen entrada no válida por U+FFFD.

## API

- [UTF-8](./utf8): `encode`, `decode` estricto y `decodeLossy` de recuperación.
- [UTF-16](./utf16): unidades de código, orden de bytes LE/BE explícito, decodificadores estrictos y decodificadores lossy.
- [Hexadecimal](./hex): codificación canónica en minúsculas y decodificación estricta.
- [Base64](./base64): alfabeto estándar `+/` con relleno obligatorio y decodificación estrictamente canónica.
- [Base64Url](./base64url): alfabeto `-_` seguro para URLs, sin relleno, y decodificación estrictamente canónica.
- [Base32](./base32): codificación canónica en mayúsculas con relleno y decodificación estrictamente canónica.
- [Radix](./radix): formato y análisis de enteros en las bases 2, 8, 10 y 16.
- [EncodingError](./errors): el payload de error devuelto por los decodificadores estrictos y el análisis radix.