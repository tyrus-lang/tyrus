---
title: StreamBody
description: Transfiere bytes HTTP de forma progresiva.
---

# StreamBody

`StreamBody` declara un cuerpo HTTP transferido progresivamente. Es opt-in: las rutas sin él conservan su `body` y sus respuestas normales en búfer.

<SignatureBlock label="Firma">StreamBody</SignatureBlock>

## Uso

Úsalo en un `body` de solicitud, en un estado de respuesta, o en ambos. El stream está limitado, por lo que un consumidor lento aplica backpressure en vez de acumular toda la transferencia en memoria.

<!-- @include: ../../../.fragments/modules/http/stream-body/usage.md -->

Las respuestas `StreamBody` usan `application/octet-stream` salvo que `reply.type(...)` proporcione otro tipo de contenido. Nunca se almacenan en búfer ni reciben un `Content-Length` sintético. Cuando la respuesta tiene un tipo de contenido comprimible y el cliente negocia Brotli o gzip, un codificador continuo comprime los chunks limitados al salir de la respuesta sin acumular el cuerpo.

## Escribir una respuesta progresivamente

Usa `reply.status(HttpStatus.X).stream()` cuando el propio handler produce los bytes con el tiempo. Devuelve un `WritableStream<u8[]>` e inicia la respuesta en cuanto se abre el writer, por lo que `await writer.write(...)` observa el backpressure del socket mientras el handler sigue ejecutándose. El estado de respuesta seleccionado debe declarar `StreamBody`.

<!-- @include: ../../../.fragments/modules/http/stream-body/writing-a-response-progressively.md -->

Después de que `stream()` abre la respuesta, escribe cada chunk mediante el writer devuelto y termínalo con `await close()`. Una escritura fallida significa que el cliente se desconectó o que la respuesta ya no pudo entregarse; rechaza con `StreamError`.

## Servir un archivo local

Un backend puede pasar un stream de archivo directamente a una respuesta `StreamBody`.

<!-- @include: ../../../.fragments/modules/http/stream-body/serving-a-local-file.md -->

`media/recording-source.bin` es la ruta local del archivo. `video/mp4` es el tipo de medio HTTP enviado al cliente; los dos valores son independientes.

## Errores

`ReadableStream.read()`, `pipeTo()` y `close()` pueden rechazar con `StreamError`. Una transferencia entrante que falla o excede el límite hace que el stream falle, así que maneja ese error cuando el handler lo lee de forma asíncrona.

::: danger error
<!-- @include: ../../../.fragments/modules/http/stream-body/errors.md -->
:::

Importa y captura `StreamError` al leer un stream que pueda fallar.
