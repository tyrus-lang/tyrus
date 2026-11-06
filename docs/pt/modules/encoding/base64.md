---
title: Base64
description: Codificação de bytes em base64 padrão com decodificação canônica e padding.
---

# Base64

<SignatureBlock label="Assinatura">base64.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Assinatura">base64.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Importe `base64` de `tyrus:encoding`. `encode(bytes)` produz a representação base64 padrão RFC 4648 do array de bytes, usando o alfabeto `+/` e padding `=` obrigatório, de modo que todo resultado não vazio tenha comprimento divisível por 4. `decode(text)` reverte a codificação estritamente: somente a forma canônica é aceita.

<!-- @include: ../../../.fragments/modules/encoding/base64-usage.md -->

Um decoder estrito rejeita tudo que o codificador canônico não poderia ter produzido com `ERR_ENCODING_INVALID_BASE64`: caracteres fora do alfabeto `+/`, padding no meio, comprimento não divisível por 4, `=` quando o resto espera um (para 2 bytes) ou dois (para 1 byte), e bits finais que não eram zero na origem. Espaços nunca são ignorados.