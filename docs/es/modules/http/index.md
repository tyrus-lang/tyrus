---
title: tyrus:http
description: APIs HTTP nativas para services tyrus y proyectos API generados.
---

# tyrus:http

`tyrus:http` es la superficie HTTP nativa para servidor y rutas. Está pensado para proyectos API generados al elegir el target API en las preguntas de `tyrus new` y para services explícitos que necesitan helpers tipados de request y response.

## Imports

<!-- @include: ../../../.fragments/modules/http/index/imports.md -->

## Ítems exportados

| Ítem | Firma | Finalidad |
|------|-------|-----------|
| [`createRouter`](./create-router.md) | `() => HttpRouter` | Crea un router HTTP nativo. |
| [API Flow](./api-flow.md) | Ejemplo completo de API User | Sigue el main, el Model, los schemas, las rutas, los controllers, los services y el repository en un solo flujo con SQLite. |
| [`OpenAPI / Swagger`](./openapi-swagger.md) | `app.docs(options)` | Genera el documento OpenAPI y sirve Swagger UI. |
| [Registro automático de rutas](./automatic-routes.md) | Descubre módulos de ruta en `src/routes/` sin registro manual. |
| [`listen`](./listen.md) | `app.listen(port, options?)` | Inicia el servidor y devuelve `Result<HttpServer, HttpError>`. El runtime lo mantiene activo después de que `main` retorna. |
| [`HttpStatus`](./http-status.md) | `enum HttpStatus` | Códigos HTTP nombrados. |
| [`HttpReply`](./http-reply.md) | `type HttpReply<T>` | API de reply tipada para controllers. |
| [`StreamBody`](./stream-body.md) | `type StreamBody` | Stream de bytes limitado y opt-in para un cuerpo o respuesta de ruta. |
| `HttpRouter` | `type HttpRouter` | Router nativo devuelto por `createRouter`. |
| [`onResponse`](./on-response.md) | `(hook: (access: HttpAccessLog) => void) => void` | Registra un observador opcional de respuestas del servidor. |
| [`HttpAccessLog`](./http-access-log.md) | `interface HttpAccessLog` | Metadatos inmutables de una respuesta terminada. |
| `HttpRequest` | `interface HttpRequest` | Solicitud sin procesar disponible para pre-handlers y rutas sin procesar. |
| [`HttpResponse`](./http-response.md) | `interface HttpResponse` | Respuesta sin procesar usada por pre-handlers y rutas sin procesar. |
| `HttpCacheOptions` | `interface HttpCacheOptions` | Configuración de ETag pasada mediante `HttpReplyOptions`. |
| `HttpReplyOptions` | `interface HttpReplyOptions` | Metadatos opcionales pasados como segundo argumento de `reply.send`. |
| [`CorsOptions`](./cors-options.md) | `interface CorsOptions` | Configuración completa pasada a `app.cors(...)`. |
| [`CsrfOptions`](./csrf-options.md) | `interface CsrfOptions` | Nombres de cookie y cabecera double-submit pasados a `app.csrf(...)`. |
| [`CookieOptions`](./cookie-options.md) | `interface CookieOptions` | Atributos opcionales pasados a `reply.setCookie(...)`. |
| [`HttpCookie`](./cookie-options.md) | `interface HttpCookie` | Una cookie de `reply.setCookies([...])`: `name`, `value` y `options` opcional. |
| [`ServerOptions`](./server-options.md) | `interface ServerOptions` | Host, límites, configuración del ciclo de vida, logging opcional y TLS que se pasan a `app.listen(...)`. |
| `HttpError` | `interface HttpError` | Error de inicio recuperable devuelto por `app.listen(...)`, con un `code` estable y un `message` legible. |
| `HttpServer` | `type HttpServer` | Servidor en ejecución devuelto por `app.listen(...)`, con `port()` y `close()`. |
| [`WebSocket`](./websocket.md) | `type WebSocket` | Una conexión WebSocket abierta, con `stream()`, `receive()`, `sendText(text)`, `sendBytes(data)`, `close()`, `protocol()` y `closeCode()`. |
| `WebSocketMessage` | `interface WebSocketMessage` | Un mensaje WebSocket completo: `binary`, y su `text` o sus `bytes`. |
| [`connectWebSocket`](./websocket.md) | `(url: string, protocols?: string[]) => Promise<Result<WebSocket, HttpError>>` | Abre una conexión WebSocket hacia una URL `ws://` o `wss://`. |
| `fetch` | `(url: string, options?: FetchOptions) => Promise<Result<HttpClientResponse, HttpError>>` | Envía una petición y lee la respuesta. |
| `HttpClientResponse` | `type HttpClientResponse` | La respuesta que recibió un `fetch`, con `status()`, `ok()`, `header(name)`, `text()`, `bytes()`, `json<T>()` y `close()`. |
| [`HttpClientStreamResponse`](./http-client-stream-response.md) | `type HttpClientStreamResponse` | La cabecera de una respuesta `fetch.stream(...)`, con `stream()` para su cuerpo progresivo. |
| `FetchOptions` | `interface FetchOptions` | Cabeceras, cuerpo de texto o stream, timeout, límite de cuerpo, número de redirecciones y descompresión de la respuesta de un `fetch`. |
| `FetchPolicyOptions` | `interface FetchPolicyOptions` | Adónde puede ir `fetch`, pasado a `app.fetchPolicy(...)`. |
| `HttpHeader` | `interface HttpHeader` | Una cabecera de petición, escrita como `{ name, value }`. |

`fetch` es el cliente. Envía una petición y lee la respuesta, y un status no es un fallo: `404` y `500` llegan como una respuesta común con `ok` falso, porque la petición alcanzó un servidor y recibió una respuesta. Un fallo de transporte devuelve `Err(HttpError)` cuando no llega ninguna respuesta, con un `code` que identifica la causa: `ERR_HTTP_FETCH_DNS` cuando el nombre no resuelve, `ERR_HTTP_FETCH_CONNECT` cuando el host rechaza, `ERR_HTTP_FETCH_TLS` cuando falla el handshake, `ERR_HTTP_FETCH_TIMEOUT`, `ERR_HTTP_FETCH_REDIRECT`, `ERR_HTTP_FETCH_BODY_TOO_LARGE` y `ERR_HTTP_FETCH_URL`. Los tres primeros están separados porque piden cosas distintas: un nombre que no resuelve suele ser un error de escritura o una configuración vieja, una conexión rechazada a menudo es transitoria y vale reintentarla, y un handshake que falla no se arregla reintentando. Las conexiones se reutilizan entre peticiones al mismo host. El cuerpo se lee en memoria cuando la petición termina, acotado por `maxBodySize` (8 MB por defecto), así que `text()` y `bytes()` son dos vistas de un cuerpo y no dos copias de él; el streaming es un contrato aparte. `json<T>()` lee ese cuerpo con el decodificador que usa un `T.fromJson(text)`, así que una respuesta y una cadena que llevan el mismo documento no pueden discrepar sobre lo que dice, ni sobre qué campo está mal cuando lo está: el fallo es el mismo `Result<T, JsonError>`, con el mismo código y la misma ruta. Las opciones se escriben en la llamada, porque `timeout` y `maxBodySize` se leen en compilación, la misma regla que siguen las opciones del servidor.

`fetch` decodifica las respuestas comprimidas por sí mismo. Salvo que el programa defina `accept-encoding`, cada petición ofrece `gzip, deflate, br`, y un cuerpo de respuesta en una de esas codificaciones llega decodificado, tanto en `fetch(...)` como en `fetch.stream(...)`, sin las cabeceras `content-encoding` y `content-length`, que describen los bytes en la red. `maxBodySize` limita el cuerpo decodificado, un cuerpo que se expande más de 200 veces su tamaño comprimido después del primer 1 MiB se rechaza con `ERR_HTTP_FETCH_RATIO_LIMIT`, y un cuerpo que no coincide con su `Content-Encoding` se rechaza con `ERR_HTTP_FETCH_DECODE`. Pasa `decompress: false` para recibir los bytes exactamente como el servidor los envió, por ejemplo para guardarlos comprimidos o decodificarlos con [`tyrus:compress`](../compress/) con límites propios.

Usa `fetch.stream(url, options?)` cuando el cuerpo deba leerse progresivamente. Devuelve `Ok(HttpClientStreamResponse)` después de que llega la cabecera de la respuesta o `Err(HttpError)` si la petición falla antes, y `await response.stream()` devuelve su `ReadableStream<u8[]>` limitado; el contrato de `fetch(...)` normal sigue estando en búfer.

Usa `bodyStream` para enviar un cuerpo de petición progresivo. Está separado del `body` textual, así que la transferencia es visible en el lugar de la llamada; el transporte toma cada chunk solo cuando puede aceptarlo, llevando el backpressure hasta la fuente. `fetch(...)` y cada verbo devuelven `Promise<Result<HttpClientResponse, HttpError>>`.

Un cuerpo de petición en stream se consume una sola vez y no se puede reenviar. Si `fetch.stream(...)` o `FetchOptions.bodyStream` recibe una redirección, el cliente conserva la respuesta 3xx y no emite una segunda petición; lea `status()` y `header("location")` si el programa quiere decidir qué hacer.

<!-- @include: ../../../.fragments/modules/http/index/exported-items.md -->

Los verbos cuelgan del propio `fetch`: `fetch.get(url)`, `fetch.post(url, options)`, y lo mismo para `put`, `patch`, `delete`, `head`, `options`, `connect`, `query` y `trace`. No se exportan por su cuenta, porque `import { get, post }` pondría esos nombres en el ámbito de todo archivo que quisiera uno, y `get` es un nombre que un programa tiene sus propios usos para. El verbo es la única forma de nombrar un método: `FetchOptions` no tiene campo `method`, así que un `fetch(url, options)` simple envía `GET` y todo lo demás es verbo. Escribir `method` de todos modos se rechaza con un mensaje que nombra el verbo a usar. Un programa es libre de nombrar un valor suyo `fetch`, y entonces los métodos son suyos.

Cada respuesta del servidor recibe una cabecera `x-request-id` con un ID de solicitud Base64URL compacto sin padding. El access log está desactivado por defecto: pase `{ logger: true }` a `app.listen(...)` para escribir un evento estructurado en stderr sin datos de la solicitud. Cuando está desactivado, el servidor no serializa ni escribe un evento de acceso. Registre `app.onResponse(...)` solo cuando el código de la aplicación necesite sus propias métricas, registro de auditoría, integración de tracing u otro trabajo posterior a la respuesta. El callback se ejecuta una vez cuando el servidor termina de producir o descarta el cuerpo, incluso un cuerpo en stream, y no puede cambiar la respuesta. Importe `Duration` de `tyrus:temporal` al acceder a `access.duration`.

## Inicio rápido

Una API REST mínima con respuestas tipadas y validación:

<!-- @include: ../../../.fragments/modules/http/index/quick-start.md -->

::: warning Mantén capas separadas
Las reglas de lint de API esperan que routes declaren ruteo, controllers traduzcan fronteras HTTP, services contengan comportamiento de negocio y repositories concentren acceso directo a la base de datos.
:::
