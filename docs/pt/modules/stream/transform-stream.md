---
title: TransformStream
description: Combine capacidades de entrada e saída com tipos diferentes.
---

# TransformStream

`TransformStream<TInput, TOutput>` expõe uma entrada gravável e uma saída legível com tipos de payload independentes. Ele não carrega codec ou função de transformação implícita. As transformações nativas que têm um, como [`compressionStream`](../compress/compression-stream.md) e [`decompressionStream`](../compress/decompression-stream.md), retornam um `TransformStream<u8[], u8[]>` cuja saída é a entrada transformada.

<SignatureBlock label="Assinatura">TransformStream&lt;TInput, TOutput&gt;</SignatureBlock>

## Membros

| Membro | Assinatura | Finalidade |
| --- | --- | --- |
| `writable` | `WritableStream<TInput>` | Envia valores de entrada. |
| `readable` | `ReadableStream<TOutput>` | Recebe valores de saída. |
| `close` | `() => Promise<void>` | Fecha os dois lados juntos. |

## Uso

<!-- @include: ../../../.fragments/modules/stream/transform-stream.md -->

## Erros

`close()` rejeita com `StreamError` se qualquer lado falhar. Os parâmetros de tipo impedem escrever valores de saída no lado de entrada ou ler valores de entrada no lado de saída.
