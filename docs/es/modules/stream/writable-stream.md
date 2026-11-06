---
title: WritableStream
description: Envía una secuencia limitada de valores progresivamente.
---

# WritableStream

`WritableStream<T>` es una capacidad de solo escritura devuelta por adaptadores nativos, como escritura de archivos y entrada de procesos hijo.

<SignatureBlock label="Firma">WritableStream&lt;T&gt;</SignatureBlock>

## Métodos

| Método | Firma | Finalidad |
| --- | --- | --- |
| `write` | `(value: T) => Promise<void>` | Espera hasta que la cola limitada acepte un valor. |
| `close` | `() => Promise<void>` | Completa escrituras aceptadas y cierra el lado escribible. |

## Uso

<!-- @include: ../../../.fragments/modules/stream/writable-stream.md -->

## Errores

`write()` rechaza con `StreamError` para un stream cerrado, fallido o usado concurrentemente, y rechaza inmediatamente un chunk binario mayor que la capacidad del buffer. `close()` es idempotente.
