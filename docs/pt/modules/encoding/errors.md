---
title: EncodingError
description: Erros dos decoders estritos de texto.
---

# EncodingError

<SignatureBlock label="Assinatura">interface EncodingError { readonly code: string; readonly message: string; }</SignatureBlock>

`EncodingError` é o payload de erro retornado pelos decoders estritos e pela análise radix em `Result<T, EncodingError>`. Trate-o com `match` ou propague-o com `?`.

Os decoders estritos de `tyrus:encoding` retornam `Err(EncodingError)` quando a entrada não está bem-formada. Seu `code` é estável para tratamento programático e sua `message` descreve a falha. Os codecs estritos e a análise radix compartilham os códigos estáveis `ERR_ENCODING_INVALID_UTF8`, `ERR_ENCODING_INVALID_UTF16`, `ERR_ENCODING_INVALID_HEX`, `ERR_ENCODING_INVALID_BASE64`, `ERR_ENCODING_INVALID_BASE32`, `ERR_ENCODING_INVALID_RADIX` e `ERR_ENCODING_INPUT_TOO_LARGE` (para estouro de radix).

<!-- @include: ../../../.fragments/modules/encoding/errors/encodingerror.md -->

Use um decoder lossy quando substituir por U+FFFD for o resultado desejado, em vez de capturar um erro.

Importe `EncodingError` pelo nome ao citá-lo numa anotação de tipo. Decoders estritos e análise radix retornam valores `Result`, que devem ser tratados com `match` ou propagados com `?`. Encoders, decoders lossy e `radix.format` são infalíveis.
