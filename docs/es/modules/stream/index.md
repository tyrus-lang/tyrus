---
title: Streams
description: Transporta secuencias limitadas de valores entre APIs nativas de Tyrus con backpressure.
---

# Streams

`tyrus:stream` define streams separados por capacidad para transportar valores progresivamente. Una capacidad de lectura no puede escribir, una capacidad de escritura no puede leer, y los streams dúplex y de transformación exponen sus dos lados explícitamente.

::: warning Estado de la integración
La capa compartida de streams está completa: sus tipos, métodos, ABI nativa, cola limitada con backpressure, ciclo de vida y soporte del editor están disponibles. Los adaptadores de filesystem son públicos mediante `readFileStream` y `writeFileStream`, los adaptadores de proceso son públicos mediante `ChildProcess`, y HTTP expone cuerpos progresivos de petición y respuesta mediante `StreamBody`, `fetch.stream(...)` y `FetchOptions.bodyStream`. TCP y WebSocket devuelven duplexes de bytes acotados mediante `TcpStream.stream()` y `WebSocket.stream()`; cada chunk de WebSocket es un mensaje binario completo.
:::

## Importaciones

<!-- @include: ../../../.fragments/modules/stream/api.md -->

## Elementos exportados

| Elemento | Firma | Finalidad |
| --- | --- | --- |
| [`ReadableStream`](./readable-stream.md) | `<T>` | Recibe valores progresivamente. |
| [`WritableStream`](./writable-stream.md) | `<T>` | Envía valores progresivamente. |
| [`DuplexStream`](./duplex-stream.md) | `<TRead, TWrite>` | Combina lados independientes de lectura y escritura. |
| [`TransformStream`](./transform-stream.md) | `<TInput, TOutput>` | Conecta un lado de entrada a una salida con tipo separado. |
| [`StreamError`](./errors.md) | `code: string, message: string` | Describe un fallo recuperable de stream. |

## Backpressure

Cada buffer de stream tiene capacidad finita. Cuando el destino no tiene espacio, `write()` permanece pendiente hasta que el consumidor lea suficientes datos. Esto evita que el uso de memoria crezca con el tamaño total de un archivo o transferencia.

En streams binarios, los chunks usan `u8[]` y la capacidad se mide en bytes. Un chunk mayor que la capacidad completa del buffer se rechaza inmediatamente, en vez de esperar un espacio que nunca podrá estar disponible.

## Lectura y cierre

`read()` se resuelve con el siguiente chunk. Después de que el escritor cierra, los chunks ya aceptados siguen disponibles para lectura; solo después de drenar la cola `read()` se resuelve con un `T?` vacío. Cerrar el lado de lectura abandona los chunks no leídos y despierta a los escritores bloqueados con un error.

El cierre es idempotente. `pipeTo()` lee desde el origen, espera cada escritura en el destino, cierra el destino después del final normal del stream y propaga los fallos sin reemplazar su error estructurado original.

## Ownership y concurrencia

Un chunk aceptado pertenece al stream hasta que `read()` lo transfiere al consumidor. Una escritura cancelada antes de ser aceptada mantiene la cola sin cambios y libera su payload pendiente. En el contrato inicial, solo una operación mutable puede estar pendiente en cada lado del stream; las operaciones concurrentes inválidas se rechazan con `StreamError`.

## Errores

Los fallos recuperables del stream usan `StreamError`. Su `code` estable sirve para comparación programática y su `message` está destinada a personas. Los errores originados en otro módulo nativo conservan el error estructurado de ese módulo al atravesar `pipeTo()`.

## Transporte binario

Video codificado, audio codificado, archivos comprimidos, documentos y archivos arbitrarios viajan como chunks opacos `u8[]`. Los streams preservan los bytes y no aplican codec, framing ni interpretación de medios.
