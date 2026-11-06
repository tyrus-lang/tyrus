---
title: StreamBody
description: Transfer HTTP request and response bytes progressively.
---

# StreamBody

`StreamBody` declares an HTTP body transferred progressively. It is opt-in: routes without it keep their ordinary buffered `body` and responses.

<SignatureBlock label="Signature">StreamBody</SignatureBlock>

## Usage

Use it on a request `body`, a response status, or both. The stream is bounded, so a slow consumer applies backpressure instead of accumulating the entire transfer in memory.

<!-- @include: ../../.fragments/modules/http/stream-body/usage.md -->

`StreamBody` responses use `application/octet-stream` unless `reply.type(...)` supplies another content type. They are never buffered or assigned a synthetic `Content-Length`. When the response has a compressible content type and the client negotiates Brotli or gzip, one continuous encoder compresses bounded chunks as they leave the response without collecting the body.

## Writing a response progressively

Use `reply.status(HttpStatus.X).stream()` when the handler itself produces the bytes over time. It returns a `WritableStream<u8[]>` and starts the response as soon as the writer is opened, so `await writer.write(...)` observes the socket's backpressure while the handler is still running. The selected response status must declare `StreamBody`.

<!-- @include: ../../.fragments/modules/http/stream-body/writing-a-response-progressively.md -->

After `stream()` opens the response, write every chunk through the returned writer and finish it with `await close()`. A failed write means the client disconnected or the response could no longer be delivered; it rejects with `StreamError`.

## Serving a local file

A backend can pass a file stream directly to a `StreamBody` response.

<!-- @include: ../../.fragments/modules/http/stream-body/serving-a-local-file.md -->

`media/recording-source.bin` is the local file path. `video/mp4` is the HTTP media type sent to the client; the two values are independent.

## Errors

`ReadableStream.read()`, `pipeTo()`, and `close()` can reject with `StreamError`. A failed or oversized incoming transfer fails the stream, so handle that error when the handler reads it asynchronously.

::: danger error
<!-- @include: ../../.fragments/modules/http/stream-body/errors.md -->
:::

Import and catch `StreamError` when reading a stream that can fail.
