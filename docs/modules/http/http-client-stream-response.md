---
title: HttpClientStreamResponse
description: HTTP client response whose body is read progressively.
---

# HttpClientStreamResponse

`HttpClientStreamResponse` is the successful value in `await fetch.stream(url, options?)`. The promise returns `Err(HttpError)` if the request fails before the status and headers arrive; after that, stream failures are reported through `StreamError`.

<SignatureBlock label="Signature">HttpClientStreamResponse</SignatureBlock>

## Methods

| Method | Signature | Purpose |
| --- | --- | --- |
| `status` | `() => i32` | Returns the HTTP status code. |
| `ok` | `() => boolean` | Returns whether the status is in the `2xx` range. |
| `header` | `(name: string) => string?` | Returns a response header without regard to its case. |
| `stream` | `() => Promise<ReadableStream<u8[]>>` | Returns the progressively readable body. |
| `close` | `() => void` | Aborts an unread body and releases the response. |

## Usage

<!-- @include: ../../.fragments/modules/http/http-client-stream-response/usage.md -->

Use ordinary `fetch(...)` when the full body is needed as `text()`, `bytes()`, or `json<T>()`. `fetch.stream(...)` keeps at most a bounded transfer buffer in memory and applies backpressure while `ReadableStream.read()` is waiting. Once `stream()` returns its reader, `close()` releases the response head without interrupting that reader; close the reader to abandon the transfer. Closing a response before obtaining a reader cancels its transfer. A transport failure before the headers arrive returns `Err(HttpError)`; a failure after that point fails the readable stream with `StreamError`.
