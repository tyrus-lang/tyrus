---
title: ReadableStream
description: Receba uma sequência limitada de valores progressivamente.
---

# ReadableStream

`ReadableStream<T>` é uma capacidade somente de leitura devolvida por adaptadores nativos, como leitura de arquivo e saída de processo filho.

<SignatureBlock label="Assinatura">ReadableStream&lt;T&gt;</SignatureBlock>

## Métodos

| Método | Assinatura | Finalidade |
| --- | --- | --- |
| `read` | `() => Promise<T?>` | Resolve com o próximo valor ou vazio no fim do stream. |
| `pipeTo` | `(destination: WritableStream<T>) => Promise<void>` | Copia valores progressivamente e fecha o destino após EOF normal. |
| `close` | `() => Promise<void>` | Abandona valores não lidos e acorda escritores bloqueados. |

## Uso

<!-- @include: ../../../.fragments/modules/stream/readable-stream.md -->

## Erros

`read()`, `pipeTo()` e `close()` rejeitam com `StreamError` quando o stream falha ou uma operação viola seu ciclo de vida. `pipeTo()` preserva o erro estruturado gerado pela origem ou destino. Cancelar uma operação `pipeTo()` pendente faz as duas pontas falharem com `ERR_STREAM_CANCELLED` e descarta os chunks em buffer.
