---
title: Raw HTTP Types
description: Use raw tyrus:http request, response, and CORS contracts.
---

# Raw HTTP Types

`HttpRequest`, `HttpResponse`, `CorsOptions`, and `CookieOptions` are explicit interfaces exported by `tyrus:http`. Import them only when working at the raw HTTP boundary. Controllers should normally use the request inferred from a route schema and send through `HttpReply`.

`HttpRequest` exposes an immutable `id`, the request method, path, and raw string channels `body`, `params`, `query`, `headers`, and `cookies`. The runtime assigns `id` once per request and sends the compact unpadded Base64URL value in the `x-request-id` response header. `{ logger: true }` writes a structured access event to stderr with the ID, method, path, status, and duration; it is disabled by default. Query text, headers, cookies, credentials, and bodies are never included in that event. Use [HttpAccessLog](./http-access-log.md) with [`app.onResponse`](./on-response.md) when application code needs that metadata. See [HttpResponse](./http-response.md), [CorsOptions](./cors-options.md), and [CookieOptions](./cookie-options.md) for their dedicated contracts.

<!-- @include: ../../.fragments/modules/http/raw-types/raw-http-types.md -->
