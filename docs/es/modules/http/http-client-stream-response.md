---
title: HttpClientStreamResponse
description: Respuesta del cliente HTTP cuyo cuerpo se lee progresivamente.
---

# HttpClientStreamResponse

`HttpClientStreamResponse` es el valor de éxito de `await fetch.stream(url, options?)`. La promise devuelve `Err(HttpError)` si la petición falla antes de que lleguen el estado y las cabeceras; después, los fallos del stream se notifican mediante `StreamError`.

<SignatureBlock label="Firma">HttpClientStreamResponse</SignatureBlock>

## Métodos

| Método | Firma | Propósito |
| --- | --- | --- |
| `status` | `() => i32` | Devuelve el código de estado HTTP. |
| `ok` | `() => boolean` | Devuelve si el estado está en el rango `2xx`. |
| `header` | `(name: string) => string?` | Devuelve una cabecera de respuesta sin considerar mayúsculas y minúsculas. |
| `stream` | `() => Promise<ReadableStream<u8[]>>` | Devuelve el cuerpo legible progresivamente. |
| `close` | `() => void` | Cancela un cuerpo no leído y libera la respuesta. |

## Uso

<!-- @include: ../../../.fragments/modules/http/http-client-stream-response/usage.md -->

Usa `fetch(...)` normal cuando se necesite el cuerpo completo como `text()`, `bytes()` o `json<T>()`. `fetch.stream(...)` conserva como máximo un búfer de transferencia limitado en memoria y aplica backpressure mientras `ReadableStream.read()` está esperando. Después de que `stream()` devuelve su lector, `close()` libera la cabecera de la respuesta sin interrumpir ese lector; cierra el lector para abandonar la transferencia. Cerrar una respuesta antes de obtener un lector cancela su transferencia. Un fallo de transporte antes de que lleguen las cabeceras devuelve `Err(HttpError)` en `fetch.stream(...)`; un fallo después de ese punto hace que el stream legible falle con `StreamError`.
