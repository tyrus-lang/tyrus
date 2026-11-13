---
title: HttpRouter.onResponse
description: Observe completed tyrus:http responses from server setup.
---

# HttpRouter.onResponse

`app.onResponse(hook)` registers a named synchronous callback for every response served by that router. Register it in server setup before `app.listen(...)`, not inside a route.

## Signature

<!-- @include: ../../.fragments/modules/http/on-response/signature.md -->

The callback runs once after the server finishes producing the response body or discards it. This includes a `StreamBody` response. It observes the final status and duration but cannot alter the response, and it is not a route `preHandler`.

<!-- @include: ../../.fragments/modules/http/on-response/signature-2.md -->

Registering more than one hook is allowed. Hooks run in registration order. The callback may perform any permitted synchronous application work, such as updating a metric, writing an audit record, or forwarding data to a tracing adapter.

::: danger error
<!-- @include: ../../.fragments/modules/http/on-response/signature-3.md -->
:::

The hook must be a named function. A named callback gives the server a stable native entry point and prevents a closure from outliving its captured route state.
