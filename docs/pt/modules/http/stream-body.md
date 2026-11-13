---
title: StreamBody
description: Transfira bytes HTTP de forma progressiva.
---

# StreamBody

`StreamBody` declara um corpo HTTP transferido progressivamente. Ele é opt-in: rotas sem ele mantêm seu `body` e suas respostas comuns em buffer.

<SignatureBlock label="Assinatura">StreamBody</SignatureBlock>

## Uso

Use-o em um `body` de requisição, em um status de resposta, ou nos dois. O stream é limitado, então um consumidor lento aplica backpressure em vez de acumular toda a transferência na memória.

<!-- @include: ../../../.fragments/modules/http/stream-body/usage.md -->

Respostas `StreamBody` usam `application/octet-stream`, a menos que `reply.type(...)` forneça outro tipo de conteúdo. Elas nunca entram em buffer nem recebem um `Content-Length` sintético. Quando a resposta tem tipo de conteúdo compressível e o cliente negocia Brotli ou gzip, um codificador contínuo comprime chunks limitados ao saírem da resposta sem acumular o corpo.

## Escrevendo uma resposta progressivamente

Use `reply.status(HttpStatus.X).stream()` quando o próprio handler produz os bytes ao longo do tempo. Ele retorna um `WritableStream<u8[]>` e inicia a resposta assim que o writer é aberto, então `await writer.write(...)` observa o backpressure do socket enquanto o handler ainda está executando. O status de resposta selecionado deve declarar `StreamBody`.

<!-- @include: ../../../.fragments/modules/http/stream-body/writing-a-response-progressively.md -->

Depois que `stream()` abre a resposta, escreva cada chunk pelo writer retornado e finalize-o com `await close()`. Uma escrita que falha significa que o cliente desconectou ou que a resposta não pôde mais ser entregue; ela rejeita com `StreamError`.

## Servindo um arquivo local

Um backend pode passar diretamente um stream de arquivo para uma resposta `StreamBody`.

<!-- @include: ../../../.fragments/modules/http/stream-body/serving-a-local-file.md -->

`media/recording-source.bin` é o caminho local do arquivo. `video/mp4` é o tipo de mídia HTTP enviado ao cliente; os dois valores são independentes.

## Erros

`ReadableStream.read()`, `pipeTo()` e `close()` podem rejeitar com `StreamError`. Uma transferência de entrada que falha ou excede o limite faz o stream falhar, então trate esse erro quando o handler o lê de modo assíncrono.

::: danger error
<!-- @include: ../../../.fragments/modules/http/stream-body/errors.md -->
:::

Importe e capture `StreamError` ao ler um stream que pode falhar.
