---
title: Hexadecimal
description: Codificação hexadecimal de bytes em minúsculas e decodificação estrita.
---

# Hexadecimal

<SignatureBlock label="Assinatura">hex.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Assinatura">hex.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Importe `hex` de `tyrus:encoding`. `encode(bytes)` produz a representação hexadecimal canônica em minúsculas do array de bytes, sem prefixo `0x`. `decode(text)` aceita somente pares completos de dígitos ASCII hexadecimais; comprimento ímpar, prefixo `0x`, espaços e caracteres inválidos retornam `Err(EncodingError)`. O decoder aceita somente o alfabeto canônico em minúsculas, portanto dígitos em maiúsculas são rejeitados em vez de normalizados silenciosamente.

<!-- @include: ../../../.fragments/modules/encoding/hex-usage.md -->

Cada par de dígitos hexadecimais nomeia exatamente um byte, então o resultado não carrega endianness: é a grafia direta em base 16 de um array de bytes.