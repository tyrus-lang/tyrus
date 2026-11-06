---
title: tyrus:compress
description: Comprime e descomprime bytes com Gzip, Deflate, Deflate cru e Brotli.
---

# tyrus:compress

`tyrus:compress` comprime e descomprime buffers e streams de bytes com Gzip, Deflate (zlib), Deflate cru e Brotli. Toda operação é assíncrona e roda fora das threads que executam as funções `async` do próprio programa, então um buffer grande nunca as trava.

A descompressão é segura por padrão: o tamanho da saída e a razão de expansão são verificados enquanto os dados são produzidos, então uma entrada pequena e maliciosa não consegue encher a memória. As opções só aumentam esses limites, nunca os desligam.

## Imports

<SignatureBlock label="Importações">import { CompressError, CompressFormat, CompressOptions, DecompressOptions, compress, compressionStream, decompress, decompressionStream } from "tyrus:compress";</SignatureBlock>

## Funções

| Item | Assinatura | Finalidade |
|------|-----------|---------|
| [`compress`](./compress.md) | `(data: u8[] \| string, format: CompressFormat, options?: CompressOptions) => Promise<Result<u8[], CompressError>>` | Comprime bytes ou os bytes UTF-8 de uma string. |
| [`decompress`](./decompress.md) | `(data: u8[], options?: DecompressOptions) => Promise<Result<u8[], CompressError>>` | Descomprime bytes, detectando Gzip e Deflate pelos dados. |
| [`compressionStream`](./compression-stream.md) | `(format: CompressFormat, options?: CompressOptions) => TransformStream<u8[], u8[]>` | Comprime uma stream de bytes de qualquer tamanho com memória constante. |
| [`decompressionStream`](./decompression-stream.md) | `(options?: DecompressOptions) => TransformStream<u8[], u8[]>` | Descomprime uma stream de bytes de qualquer tamanho com memória constante. |

## Formatos

| `CompressFormat` | Formato | Detectado por `decompress` | Níveis |
|------------------|--------|--------------------------|--------|
| `Gzip` | Gzip (`.gz`), inclusive vários membros concatenados | Sim | `0` a `9`, padrão `6` |
| `Deflate` | zlib, o `deflate` do cabeçalho HTTP `Content-Encoding` | Sim | `0` a `9`, padrão `6` |
| `DeflateRaw` | Deflate sem cabeçalho nem checksum | Não | `0` a `9`, padrão `6` |
| `Brotli` | Brotli | Não | `0` a `11`, padrão `6` |

A mesma entrada, formato e nível sempre produzem os mesmos bytes. Brotli e Deflate cru não têm assinatura, então `decompress` precisa de `format` para eles em vez de adivinhar.

## Códigos de erro

Toda função resolve com um `Result` e nunca rejeita: o sucesso é `Ok` com os bytes, e uma falha é `Err` com um [`CompressError`](./types.md#compresserror). Seu `code` é estável e sua `message` é legível. Dentro de uma função que retorna um `Result` com o mesmo erro, `await compress(...)?` entrega os bytes ou retorna o erro, e `onfail` o trata no próprio lugar.

| Código | Significado |
|------|---------|
| `ERR_COMPRESS_INVALID_DATA` | Os dados estão malformados, truncados, têm bytes depois do fim ou não estão no formato informado. |
| `ERR_COMPRESS_CHECKSUM_MISMATCH` | Os dados Gzip ou zlib não conferem com o checksum ou com o tamanho registrado. |
| `ERR_COMPRESS_UNKNOWN_FORMAT` | `format` foi omitido e os dados não têm assinatura Gzip nem zlib. |
| `ERR_COMPRESS_INVALID_LEVEL` | `level` está fora do intervalo do formato. |
| `ERR_COMPRESS_INVALID_OPTION` | Uma opção tem valor inválido, como um limite `0`. |
| `ERR_COMPRESS_OUTPUT_LIMIT` | A saída descomprimida passaria de `maxOutputBytes`. |
| `ERR_COMPRESS_RATIO_LIMIT` | A saída descomprimida passaria de `maxRatio` vezes a entrada. |
| `ERR_COMPRESS_ALLOCATION_FAILED` | Não foi possível alocar memória para a saída. |

## Exemplo

<!-- @include: ../../../.fragments/modules/compress/index/example.md -->
