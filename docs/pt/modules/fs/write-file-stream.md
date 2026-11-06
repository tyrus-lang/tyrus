---
title: writeFileStream
description: Escreva chunks binários limitados progressivamente em um arquivo.
---

# writeFileStream

`writeFileStream` cria ou trunca um arquivo e retorna um `Result<WritableStream<u8[]>, FsError>` limitado. O backpressure impede que produtores mantenham toda a saída na memória.

## Assinatura

<!-- @include: ../../../.fragments/modules/fs/write-file-stream/signature.md -->

A fila comporta no máximo 256 KiB. `await stream.close()` e `await source.pipeTo(stream)` resolvem somente depois que todos os chunks aceitos foram escritos e o buffer do arquivo foi descarregado.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/write-file-stream/basic-usage.md -->

Falhas ao abrir retornam `Err(FsError)`. Falhas de escrita ou flush rejeitam operações do stream com `StreamError`.
