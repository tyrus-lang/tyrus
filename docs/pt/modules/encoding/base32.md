---
title: Base32
description: Codificação de bytes em base32 canônica em maiúsculas, com padding e decodificação estrita.
---

# Base32

<SignatureBlock label="Assinatura">base32.encode(bytes: u8[]): string</SignatureBlock>

<SignatureBlock label="Assinatura">base32.decode(text: string): Result&lt;u8[], EncodingError&gt;</SignatureBlock>

Importe `base32` de `tyrus:encoding`. `encode(bytes)` produz a representação RFC 4648 base32 canônica em maiúsculas, usando o alfabeto `A`-`Z` e `2`-`7` com padding `=` obrigatório, de modo que todo resultado não vazio tenha comprimento divisível por 8. `decode(text)` reverte a codificação estritamente: somente a forma canônica é aceita.

<!-- @include: ../../../.fragments/modules/encoding/base32-usage.md -->

Um decoder estrito rejeita tudo que o codificador canônico não poderia ter produzido com `ERR_ENCODING_INVALID_BASE32`: letras minúsculas, caracteres fora do alfabeto `A`-`Z`/`2`-`7`, comprimento não divisível por 8, `=` no meio, contagem de padding que não corresponde ao resto de bytes, e bits finais que não eram zero na origem. Espaços nunca são ignorados. Variantes como Crockford Base32 ou base32 sem padding não fazem parte deste módulo.