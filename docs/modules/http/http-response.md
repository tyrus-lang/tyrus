---
title: HttpResponse
description: Raw HTTP response used by synchronous pre-handlers.
---

# HttpResponse

`HttpResponse` is the raw response returned by a synchronous `preHandler` when it must stop the request before the controller runs. It carries a complete early HTTP response; normal controllers should use `HttpReply` instead.

It contains `statusCode: HttpStatus`, `body: string | u8[]`, optional `contentType: string`, and `cookies: string`. The body is sent as supplied: text is not JSON-serialized and bytes support files or other binary payloads. Without `contentType`, text uses `text/plain; charset=utf-8` and bytes use `application/octet-stream`. Set `cookies` to an empty string when the pre-handler does not need to emit cookies.

Import `HttpResponse` explicitly together with `HttpRequest` when declaring a pre-handler.

<!-- @include: ../../.fragments/modules/http/http-response/httpresponse.md -->
