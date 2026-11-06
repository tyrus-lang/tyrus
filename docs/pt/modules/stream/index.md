---
title: Streams
description: Transporte sequências limitadas de valores entre APIs nativas do Tyrus com backpressure.
---

# Streams

`tyrus:stream` define streams separados por capacidade para transportar valores progressivamente. Uma capacidade de leitura não pode escrever, uma capacidade de escrita não pode ler, e streams duplex e de transformação expõem seus dois lados explicitamente.

::: warning Estado da integração
A camada compartilhada de streams está concluída: seus tipos, métodos, ABI nativa, fila limitada com backpressure, ciclo de vida e suporte do editor estão disponíveis. Os adaptadores de filesystem são públicos por meio de `readFileStream` e `writeFileStream`, os adaptadores de processo são públicos por meio de `ChildProcess`, e HTTP expõe corpos progressivos de requisição e resposta por `StreamBody`, `fetch.stream(...)` e `FetchOptions.bodyStream`. TCP e WebSocket retornam duplexes de bytes limitados por `TcpStream.stream()` e `WebSocket.stream()`; cada chunk de WebSocket é uma mensagem binária completa.
:::

## Imports

<!-- @include: ../../../.fragments/modules/stream/api.md -->

## Itens exportados

| Item | Assinatura | Finalidade |
| --- | --- | --- |
| [`ReadableStream`](./readable-stream.md) | `<T>` | Recebe valores progressivamente. |
| [`WritableStream`](./writable-stream.md) | `<T>` | Envia valores progressivamente. |
| [`DuplexStream`](./duplex-stream.md) | `<TRead, TWrite>` | Combina lados independentes de leitura e escrita. |
| [`TransformStream`](./transform-stream.md) | `<TInput, TOutput>` | Conecta um lado de entrada a uma saída com tipo separado. |
| [`StreamError`](./errors.md) | `code: string, message: string` | Descreve uma falha recuperável de stream. |

## Backpressure

Todo buffer de stream tem capacidade finita. Quando o destino não tem espaço, `write()` permanece pendente até que o consumidor leia dados suficientes. Isso impede que o uso de memória cresça com o tamanho total de um arquivo ou transferência.

Em streams binários, chunks usam `u8[]` e a capacidade é medida em bytes. Um chunk maior que a capacidade completa do buffer é rejeitado imediatamente, em vez de esperar por um espaço que nunca poderá ficar disponível.

## Leitura e fechamento

`read()` resolve com o próximo chunk. Depois que o escritor fecha, chunks já aceitos continuam disponíveis para leitura; somente após drenar a fila `read()` resolve com um `T?` vazio. Fechar o lado de leitura abandona chunks não lidos e acorda escritores bloqueados com um erro.

O fechamento é idempotente. `pipeTo()` lê da origem, espera cada escrita no destino, fecha o destino após o fim normal do stream e propaga falhas sem substituir o erro estruturado original.

## Ownership e concorrência

Um chunk aceito pertence ao stream até que `read()` o transfira ao consumidor. Uma escrita cancelada antes de ser aceita mantém a fila inalterada e libera seu payload pendente. No contrato inicial, somente uma operação mutável pode estar pendente em cada lado do stream; operações concorrentes inválidas rejeitam com `StreamError`.

## Erros

Falhas recuperáveis do stream usam `StreamError`. Seu `code` estável é adequado para comparação programática e sua `message` é destinada a pessoas. Erros originados em outro módulo nativo preservam o erro estruturado desse módulo ao atravessar `pipeTo()`.

## Transporte binário

Vídeo codificado, áudio codificado, arquivos compactados, documentos e arquivos arbitrários trafegam como chunks opacos `u8[]`. Streams preservam os bytes e não aplicam codec, framing ou interpretação de mídia.
