---
title: DuplexStream
description: Combina capacidades independientes de lectura y escritura.
---

# DuplexStream

`DuplexStream<TRead, TWrite>` agrupa un lado legible y un lado escribible sin conceder permisos adicionales a ninguna capacidad. Una conexión TCP se convierte en uno mediante `TcpStream.stream()`, y una conexión WebSocket binaria mediante `WebSocket.stream()`, ambas como `DuplexStream<u8[], u8[]>`. Los chunks TCP son bytes de transporte; cada chunk WebSocket es un mensaje binario completo.

<SignatureBlock label="Firma">DuplexStream&lt;TRead, TWrite&gt;</SignatureBlock>

## Miembros

| Miembro | Firma | Finalidad |
| --- | --- | --- |
| `readable` | `ReadableStream<TRead>` | Recibe valores. |
| `writable` | `WritableStream<TWrite>` | Envía valores. |
| `close` | `() => Promise<void>` | Cierra ambos lados juntos. |

## Uso

<!-- @include: ../../../.fragments/modules/stream/duplex-stream.md -->

## Errores

`close()` rechaza con `StreamError` si cualquiera de los lados no puede completar su ciclo de vida. Los fallos de lectura y escritura permanecen aislados en su capacidad correspondiente.
