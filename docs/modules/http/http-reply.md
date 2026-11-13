---
title: HttpReply
description: Typed HTTP reply API for tyrus controllers.
---

# HttpReply

`HttpReply<T>` is the typed response object used by HTTP controllers.

Declare application error bodies with required `code: string` and `message: string` fields. The handler must send both fields, for example `{ code: "user_not_found", message: "User not found" }`; Tyrus does not derive or overwrite `code` from the HTTP status. OpenAPI documents the fields declared by the application.

Controllers may return `void` or `Promise<void>`. Async inline controllers infer `Promise<void>` when the return annotation is omitted, and the HTTP runtime waits for settlement before finalizing the response. The single-send and complete-success-path checks include code on both sides of `await`.

## Usage

<!-- @include: ../../.fragments/modules/http/http-reply/usage.md -->
Error response schemas belong to the application. A service should return domain data or throw a domain error; a controller or future error boundary maps it to an HTTP status and this declared response body.

## Headers and cookies

Prepare response metadata before `status(...).send(...)`. `header(name, value)` replaces a header, `appendHeader(name, value)` adds another value, `removeHeader(name)` removes values set earlier in the reply, and `type(value)` sets `Content-Type`.

<!-- @include: ../../.fragments/modules/http/http-reply/headers-and-cookies.md -->

Header names and values are validated at the native boundary. Headers that control message framing (`connection`, `content-length`, and `transfer-encoding`) and `set-cookie` cannot be changed through the generic header API. Use `setCookie(name, value, options?)` and `clearCookie` for cookies. `setCookies(cookies)` sets a list of `HttpCookie` values (`name`, `value`, `options?`) at once, and it sets all of them or none: when any entry is rejected, no cookie from the list is written. `clearCookies()` expires every cookie the request sent; a server only knows those, so a request without cookies leaves the reply unchanged. Cookie options can set `domain`, `path`, `maxAge`, `secure`, `httpOnly`, and `sameSite`; `sameSite: "none"` requires `secure: true`. JSON is the default content type when `type(...)` is not called.

A `send` or `stream` must be preceded by `status(...)`: the response slot starts at `500`, so sending without choosing a status would serve the success body under a failure status. The checker reports that at the final reply method itself. `stream()` is available only for a status declared as `StreamBody`; it returns a `WritableStream<u8[]>` that begins the response progressively. See [`StreamBody`](./stream-body.md) for its signature and lifecycle.

Header and cookie names and values are validated. A name written as a literal is checked at compile time, with the error pointing at the literal; a name built at run time is checked at the native boundary, where a rejection is written to stderr and the header or cookie is not sent. Framing headers (`connection`, `content-length`, `transfer-encoding`) and `set-cookie` are protected: use the cookie methods for cookies.

## Conditional caching

Pass cache metadata as the optional second argument of `send`. `HttpCacheOptions.etag` is the opaque validator value; Tyrus quotes and validates it. Set `weak: true` only when the validator is intentionally weak. `lastModified` accepts an `Instant` and is formatted as an HTTP date, while `control` becomes `Cache-Control`. When `If-None-Match` contains the same tag, including its weak form or a tag in a comma-separated list, Tyrus returns `304 Not Modified`, preserves the ETag, and omits the body.

Use a valid HTTP date in `last-modified` when the representation has a meaningful modification time. In the absence of `If-None-Match`, `If-Modified-Since` at or after that time also produces `304 Not Modified`. `If-None-Match` always has precedence.

## Response compression

Tyrus negotiates `br` or `gzip` automatically for buffered, compressible responses of at least 1024 bytes when the request accepts them. Brotli wins when both have the same quality. A compressed response includes its `Content-Encoding` and `Vary: Accept-Encoding`. It never recompresses an explicit `Content-Encoding`, formats that are already compressed, or a response with a strong ETag, because a strong validator identifies one exact representation. Streaming compression is planned separately.

<!-- @include: ../../.fragments/modules/http/http-reply/response-compression.md -->

<!-- @include: ../../.fragments/modules/http/http-reply/response-compression-2.md -->

`controller-must-send-response` validates that controller success paths send a response.
