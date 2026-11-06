---
title: EncodingError
description: Errores de los decodificadores estrictos de texto.
---

# EncodingError

<SignatureBlock label="Firma">interface EncodingError { readonly code: string; readonly message: string; }</SignatureBlock>

`EncodingError` es el payload de error que devuelven los decodificadores estrictos y el análisis radix en `Result<T, EncodingError>`. Manéjelo con `match` o propáguelo con `?`.

Los decodificadores estrictos de `tyrus:encoding` devuelven `Err(EncodingError)` cuando la entrada no está bien formada. Su `code` es estable para el manejo programático y su `message` describe el fallo. Los codecs estrictos y el análisis radix comparten los códigos estables `ERR_ENCODING_INVALID_UTF8`, `ERR_ENCODING_INVALID_UTF16`, `ERR_ENCODING_INVALID_HEX`, `ERR_ENCODING_INVALID_BASE64`, `ERR_ENCODING_INVALID_BASE32`, `ERR_ENCODING_INVALID_RADIX` y `ERR_ENCODING_INPUT_TOO_LARGE` (para desbordamiento de radix).

<!-- @include: ../../../.fragments/modules/encoding/errors/encodingerror.md -->

Use un decodificador lossy cuando sustituir por U+FFFD sea el resultado deseado, en vez de capturar un error.

Importe `EncodingError` por nombre cuando lo mencione en una anotación de tipo. Los decodificadores estrictos y el análisis radix devuelven valores `Result`, que se deben manejar con `match` o propagar con `?`. Los codificadores, los decodificadores lossy y `radix.format` son infalibles.
