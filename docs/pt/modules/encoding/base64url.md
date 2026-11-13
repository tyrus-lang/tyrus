---
title: Base64Url
description: Codificação de bytes em base64 segura para URLs, sem padding, e decodificação estrita.
---

# Base64Url

<SignatureBlock label="Assinatura">base64Url.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Assinatura">base64Url.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Importe `base64Url` de `tyrus:encoding`. `encode(bytes)` produz a representação base64 RFC 4648 segura para URLs, substituindo `+` por `-` e `/` por `_` e omitindo o padding `=`. O alfabeto não contém caracteres que exijam percent-encoding em URLs ou dentro de strings JSON, e a forma sem padding permanece a mais curta possível. `decode(text)` reverte a codificação estritamente: padding e caracteres do alfabeto padrão são rejeitados com `ERR_ENCODING_INVALID_BASE64`.

<!-- @include: ../../../.fragments/modules/encoding/base64url-usage.md -->

Como o padding é omitido, o comprimento de uma entrada válida nunca é 1 módulo 4; qualquer outro comprimento é rejeitado. Diferente do alfabeto padrão, a variante segura para URLs não é autodelimitadora, por isso `encode` e `decode` são inversos exatos um do outro e nunca aceitam entrada com `=`.