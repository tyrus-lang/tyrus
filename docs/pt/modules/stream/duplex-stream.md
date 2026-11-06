---
title: DuplexStream
description: Combine capacidades independentes de leitura e escrita.
---

# DuplexStream

`DuplexStream<TRead, TWrite>` agrupa um lado legível e um lado gravável sem conceder permissões extras a nenhuma capacidade. Uma conexão TCP vira um por meio de `TcpStream.stream()`, e uma conexão WebSocket binária por `WebSocket.stream()`, ambas como `DuplexStream<u8[], u8[]>`. Chunks TCP são bytes de transporte; cada chunk WebSocket é uma mensagem binária completa.

<SignatureBlock label="Assinatura">DuplexStream&lt;TRead, TWrite&gt;</SignatureBlock>

## Membros

| Membro | Assinatura | Finalidade |
| --- | --- | --- |
| `readable` | `ReadableStream<TRead>` | Recebe valores. |
| `writable` | `WritableStream<TWrite>` | Envia valores. |
| `close` | `() => Promise<void>` | Fecha os dois lados juntos. |

## Uso

<!-- @include: ../../../.fragments/modules/stream/duplex-stream.md -->

## Erros

`close()` rejeita com `StreamError` se qualquer lado não puder concluir seu ciclo de vida. Falhas de leitura e escrita permanecem isoladas em sua capacidade correspondente.
