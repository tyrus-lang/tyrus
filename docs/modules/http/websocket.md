---
title: WebSocket
description: WebSocket routes and clients with whole messages and backpressure.
---

# WebSocket

A WebSocket is a long-lived, two-way connection. `app.ws(path, options)` turns a route into one, and `connectWebSocket(url, protocols?)` returns `Promise<Result<WebSocket, HttpError>>`. Both ends get the same `WebSocket` value, so reading and writing look the same on either side.

## Route options

| Option | Type | Purpose |
| --- | --- | --- |
| `handler` | `(socket: WebSocket) => Promise<void>` | Required. Runs once per connection; the connection closes when its Promise settles. |
| `origins` | `string[]` | Browser origins allowed to connect. Without it only the server's own origin is accepted; a request with no `Origin` header, which does not come from a browser page, is always accepted. |
| `subprotocols` | `string[]` | Subprotocols the route speaks, in order of preference. |
| `maxMessageBytes` | `i64` | Largest message accepted, 1 MB by default. A larger message fails the `receive` that reads it. |
| `preHandler` | `((request: HttpRequest) => HttpResponse \| true)[]` | Named functions run against the upgrade request before it is accepted, where cookies and headers decide who may connect; the first one that returns a response refuses the upgrade with it. Upgrades also count against the server's `rateLimit`. |
| `config` | `{ rateLimit?: RateLimit }` | A limit on upgrades for this route, written like an HTTP route's `config.rateLimit`; it replaces the server-wide `rateLimit` for this route. |

## Methods

| Method | Signature | Purpose |
| --- | --- | --- |
| `stream` | `() => Promise<Result<DuplexStream<u8[], u8[]>, HttpError>>` | Transfers binary message I/O to a bounded duplex. Each chunk is one complete binary WebSocket message. |
| `receive` | `() => Promise<Result<WebSocketMessage?, HttpError>>` | Waits for the next whole message; resolves with no message once the peer has closed the connection. |
| `sendText` | `(text: string) => Promise<Result<void, HttpError>>` | Sends one text message and resolves once it has been written. |
| `sendBytes` | `(data: u8[]) => Promise<Result<void, HttpError>>` | Sends one binary message and resolves once it has been written. |
| `close` | `(code?: i32, reason?: string) => Promise<Result<void, HttpError>>` | Sends a close with `code` (1000 by default, or 3000 to 4999) and an optional `reason` of at most 123 bytes, and ends the connection; closing twice is not an error. |
| `protocol` | `() => string` | The agreed subprotocol, or an empty string. |
| `closeCode` | `() => i32?` | The close code received from the peer after `receive()` observes its close frame, or no value before then or after an unclean transport end. |

`WebSocketMessage` carries one whole message, never a piece of one: `binary` says which field holds it, `text` for a text message and `bytes` for a binary one. Pings are answered automatically and never reach the program. The runtime also sends a heartbeat ping every 30 seconds; if the peer misses the following probe, the socket ends and a pending `receive()` resolves with no message.

`stream()` hands binary message I/O to a `DuplexStream<u8[], u8[]>`. Its readable and writable chunks are each one complete binary WebSocket message, never a fragment. The handoff makes `receive()`, `sendText()` and `sendBytes()` return `Err(HttpError)` with `ERR_HTTP_WS_STREAMED`, so they cannot race the duplex. Closing its writable side starts the close handshake while the readable side can still receive the peer's final messages. A received text message cannot become bytes silently: it fails the readable side with `StreamError` code `ERR_STREAM_WS_TEXT_MESSAGE`.

A send returns `Promise<Result<void, HttpError>>` and waits for the connection to accept the message, so a fast sender slows down to the pace of a slow peer instead of queueing without limit. Only one `receive` may be pending at a time. `server.close()` closes every open socket with code 1001 and waits for its handler, within `shutdownTimeout`; a `receive` the handler has pending resolves with no message.

## Errors

All fallible WebSocket methods return recoverable failures as `Err(HttpError)`. Codes include `ERR_HTTP_WS_CLOSED`, `ERR_HTTP_WS_STREAMED`, `ERR_HTTP_WS_BUSY`, `ERR_HTTP_WS_SEND`, `ERR_HTTP_WS_CLOSE_CODE` and `ERR_HTTP_WS_CLOSE_REASON`; message receive errors include `ERR_HTTP_WS_MESSAGE_TOO_LARGE` and `ERR_HTTP_WS_RECEIVE`. Connection failures return `Err(HttpError)` from `connectWebSocket`, with codes `ERR_HTTP_WS_URL`, `ERR_HTTP_WS_DNS`, `ERR_HTTP_WS_CONNECT`, `ERR_HTTP_WS_TLS`, `ERR_HTTP_WS_HANDSHAKE`, or `ERR_HTTP_FETCH_BLOCKED` when `app.fetchPolicy(...)` refuses the destination, which it judges the same way it judges `fetch`.

## Usage

<!-- @include: ../../.fragments/modules/http/websocket/usage.md -->
