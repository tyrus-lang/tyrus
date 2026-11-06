---
title: WebSocket
description: Rutas y clientes WebSocket con mensajes completos y backpressure.
---

# WebSocket

Un WebSocket es una conexión bidireccional de larga duración. `app.ws(path, options)` convierte una ruta en una, y `connectWebSocket(url, protocols?)` devuelve `Promise<Result<WebSocket, HttpError>>`. Los dos extremos reciben el mismo valor `WebSocket`, así que leer y escribir es igual de ambos lados.

## Opciones de la ruta

| Opción | Tipo | Finalidad |
| --- | --- | --- |
| `handler` | `(socket: WebSocket) => Promise<void>` | Obligatorio. Se ejecuta una vez por conexión; la conexión se cierra cuando su Promise termina. |
| `origins` | `string[]` | Orígenes de navegador que pueden conectarse. Sin ella solo se acepta el propio origen del servidor; una petición sin cabecera `Origin`, que no viene de una página de navegador, siempre se acepta. |
| `subprotocols` | `string[]` | Subprotocolos que habla la ruta, en orden de preferencia. |
| `maxMessageBytes` | `i64` | Mensaje más grande aceptado, 1 MB por defecto. Un mensaje mayor hace fallar el `receive` que lo lee. |
| `preHandler` | `((request: HttpRequest) => HttpResponse \| true)[]` | Funciones nombradas ejecutadas sobre la petición de upgrade antes de aceptarla, donde cookies y cabeceras deciden quién puede conectarse; la primera que devuelva una respuesta rechaza el upgrade con ella. Los upgrades también cuentan en el `rateLimit` del servidor. |
| `config` | `{ rateLimit?: RateLimit }` | Un límite de upgrades para esta ruta, escrito como el `config.rateLimit` de una ruta HTTP; reemplaza el `rateLimit` del servidor para esta ruta. |

## Métodos

| Método | Firma | Finalidad |
| --- | --- | --- |
| `stream` | `() => Promise<Result<DuplexStream<u8[], u8[]>, HttpError>>` | Transfiere la I/O de mensajes binarios a un duplex acotado. Cada chunk es un mensaje WebSocket binario completo. |
| `receive` | `() => Promise<Result<WebSocketMessage?, HttpError>>` | Espera el siguiente mensaje completo; se resuelve sin mensaje cuando el otro extremo cerró la conexión. |
| `sendText` | `(text: string) => Promise<Result<void, HttpError>>` | Envía un mensaje de texto y se resuelve cuando fue escrito. |
| `sendBytes` | `(data: u8[]) => Promise<Result<void, HttpError>>` | Envía un mensaje binario y se resuelve cuando fue escrito. |
| `close` | `(code?: i32, reason?: string) => Promise<Result<void, HttpError>>` | Envía un cierre con `code` (1000 por defecto, o de 3000 a 4999) y un `reason` opcional de hasta 123 bytes, y termina la conexión; cerrar dos veces no es un error. |
| `protocol` | `() => string` | El subprotocolo acordado, o una cadena vacía. |
| `closeCode` | `() => i32?` | El código de cierre recibido del otro extremo después de que `receive()` observa su frame de cierre, o sin valor antes de eso o tras un final de transporte abrupto. |

`WebSocketMessage` lleva un mensaje completo, nunca un fragmento: `binary` indica qué campo lo contiene, `text` para un mensaje de texto y `bytes` para uno binario. Los pings se responden automáticamente y nunca llegan al programa. El runtime también envía un ping de heartbeat cada 30 segundos; si el otro extremo no responde a la siguiente sonda, el socket termina y un `receive()` pendiente se resuelve sin mensaje.

`stream()` transfiere la I/O de mensajes binarios a un `DuplexStream<u8[], u8[]>`. Sus chunks legibles y escribibles son cada uno un mensaje WebSocket binario completo, nunca un fragmento. La transferencia hace que `receive()`, `sendText()` y `sendBytes()` devuelvan `Err(HttpError)` con `ERR_HTTP_WS_STREAMED`, para que no compitan con el duplex. Cerrar su lado escribible inicia el handshake de cierre mientras el lado legible todavía puede recibir los mensajes finales del otro extremo. Un mensaje de texto recibido no puede convertirse silenciosamente en bytes: falla el lado legible con el código `ERR_STREAM_WS_TEXT_MESSAGE` de `StreamError`.

Un envío devuelve `Promise<Result<void, HttpError>>` y espera a que la conexión acepte el mensaje, así que un emisor rápido se adapta al ritmo de un receptor lento en lugar de encolar sin límite. Solo puede haber un `receive` pendiente a la vez. `server.close()` cierra todo socket abierto con el código 1001 y espera a su handler, dentro de `shutdownTimeout`; un `receive` pendiente en el handler se resuelve sin mensaje.

## Errores

Todos los métodos falibles de WebSocket devuelven fallos recuperables como `Err(HttpError)`. Los códigos incluyen `ERR_HTTP_WS_CLOSED`, `ERR_HTTP_WS_STREAMED`, `ERR_HTTP_WS_BUSY`, `ERR_HTTP_WS_SEND`, `ERR_HTTP_WS_CLOSE_CODE` y `ERR_HTTP_WS_CLOSE_REASON`; los errores de recepción incluyen `ERR_HTTP_WS_MESSAGE_TOO_LARGE` y `ERR_HTTP_WS_RECEIVE. Los fallos de conexión devuelven `Err(HttpError)` desde `connectWebSocket`, con los códigos `ERR_HTTP_WS_URL`, `ERR_HTTP_WS_DNS`, `ERR_HTTP_WS_CONNECT`, `ERR_HTTP_WS_TLS`, `ERR_HTTP_WS_HANDSHAKE` o `ERR_HTTP_FETCH_BLOCKED` cuando `app.fetchPolicy(...)` rechaza el destino, que juzga igual que juzga `fetch`.

## Uso

<!-- @include: ../../../.fragments/modules/http/websocket/usage.md -->
