---
title: TransformStream
description: Combina capacidades de entrada y salida con tipos distintos.
---

# TransformStream

`TransformStream<TInput, TOutput>` expone una entrada escribible y una salida legible con tipos de payload independientes. No contiene un códec ni una función de transformación implícita. Las transformaciones nativas que sí lo tienen, como [`compressionStream`](../compress/compression-stream.md) y [`decompressionStream`](../compress/decompression-stream.md), devuelven un `TransformStream<u8[], u8[]>` cuya salida es la entrada transformada.

<SignatureBlock label="Firma">TransformStream&lt;TInput, TOutput&gt;</SignatureBlock>

## Miembros

| Miembro | Firma | Finalidad |
| --- | --- | --- |
| `writable` | `WritableStream<TInput>` | Envía valores de entrada. |
| `readable` | `ReadableStream<TOutput>` | Recibe valores de salida. |
| `close` | `() => Promise<void>` | Cierra ambos lados juntos. |

## Uso

<!-- @include: ../../../.fragments/modules/stream/transform-stream.md -->

## Errores

`close()` rechaza con `StreamError` si cualquiera de los lados falla. Los parámetros de tipo impiden escribir valores de salida en el lado de entrada o leer valores de entrada desde el lado de salida.
