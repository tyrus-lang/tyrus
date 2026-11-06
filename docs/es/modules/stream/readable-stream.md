---
title: ReadableStream
description: Recibe una secuencia limitada de valores progresivamente.
---

# ReadableStream

`ReadableStream<T>` es una capacidad de solo lectura devuelta por adaptadores nativos, como lectura de archivos y salida de procesos hijo.

<SignatureBlock label="Firma">ReadableStream&lt;T&gt;</SignatureBlock>

## Métodos

| Método | Firma | Finalidad |
| --- | --- | --- |
| `read` | `() => Promise<T?>` | Se resuelve con el siguiente valor o vacío al final del stream. |
| `pipeTo` | `(destination: WritableStream<T>) => Promise<void>` | Copia valores progresivamente y cierra el destino después de EOF normal. |
| `close` | `() => Promise<void>` | Abandona valores no leídos y despierta escritores bloqueados. |

## Uso

<!-- @include: ../../../.fragments/modules/stream/readable-stream.md -->

## Errores

`read()`, `pipeTo()` y `close()` rechazan con `StreamError` cuando el stream ha fallado o una operación viola su ciclo de vida. `pipeTo()` conserva el error estructurado producido por su origen o destino. Cancelar una operación `pipeTo()` pendiente hace que ambos extremos fallen con `ERR_STREAM_CANCELLED` y descarta los fragmentos almacenados en el búfer.
