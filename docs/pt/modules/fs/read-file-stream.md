---
title: readFileStream
description: Leia um arquivo progressivamente como chunks binários limitados.
---

# readFileStream

`readFileStream` abre um arquivo e retorna um `Result<ReadableStream<u8[]>, FsError>` limitado. O runtime lê chunks progressivamente, portanto o uso de memória não cresce com o tamanho completo do arquivo.

## Assinatura

<!-- @include: ../../../.fragments/modules/fs/read-file-stream/signature.md -->

O tamanho padrão de chunk é 64 KiB e a fila limitada comporta no máximo 256 KiB. Cada chunk preserva os bytes originais, incluindo bytes zero e dados que não são texto válido.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/read-file-stream/basic-usage.md -->

Falhas ao abrir retornam `Err(FsError)`. Falhas posteriores à abertura do arquivo rejeitam operações do stream com `StreamError`.
