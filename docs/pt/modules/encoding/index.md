---
title: Encoding
description: Conversão explícita de UTF-8, UTF-16, hexadecimal, Base64, Base32 e radix.
---

# Encoding

`tyrus:encoding` mantém `string` em UTF-8 e expõe conversão explícita nos limites de dados. Importe `utf8` para bytes UTF-8, `utf16` para unidades UTF-16 ou sequências de bytes little-endian e big-endian, `hex` para codificação hexadecimal canônica em minúsculas, `base64` e `base64Url` para base64 padrão ou segura para URLs, `base32` para base32 canônica em maiúsculas, e `radix` para formatação e análise de inteiros nas bases 2, 8, 10 e 16. Os decoders estritos e a análise radix retornam `Result<..., EncodingError>`, que pode ser tratada com `match` ou propagada com `?`; encoders, `radix.format` e os decoders `decodeLossy`, `decodeLELossy` e `decodeBELossy` são infalíveis, e os decoders lossy substituem entrada inválida por U+FFFD.

## API

- [UTF-8](./utf8): `encode`, `decode` estrito e `decodeLossy` de recuperação.
- [UTF-16](./utf16): unidades de código, ordem de bytes LE/BE explícita, decoders estritos e decoders lossy.
- [Hexadecimal](./hex): codificação canônica em minúsculas e decodificação estrita.
- [Base64](./base64): alfabeto padrão `+/` com padding obrigatório e decodificação estritamente canônica.
- [Base64Url](./base64url): alfabeto `-_` seguro para URLs, sem padding, e decodificação estritamente canônica.
- [Base32](./base32): codificação canônica em maiúsculas com padding e decodificação estritamente canônica.
- [Radix](./radix): formatação e análise de inteiros nas bases 2, 8, 10 e 16.
- [EncodingError](./errors): o payload de erro retornado pelos decoders estritos e pela análise radix.