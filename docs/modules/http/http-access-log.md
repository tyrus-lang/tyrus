---
title: HttpAccessLog
description: Immutable request and response metadata for tyrus:http response hooks.
---

# HttpAccessLog

`HttpAccessLog` is the immutable event received by an `app.onResponse(...)` callback. It contains only routing and response metadata. It never includes a query string, headers, cookies, credentials, or body content.

## Signature

<!-- @include: ../../.fragments/modules/http/http-access-log/signature.md -->

`requestId` is also sent to the client in the `x-request-id` response header. `duration` is the elapsed server time until the response body finishes or is discarded, represented as a `Duration` from `tyrus:temporal`.

<!-- @include: ../../.fragments/modules/http/http-access-log/signature-2.md -->
