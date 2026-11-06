---
title: CorsOptions
description: Complete CORS configuration for an HTTP router.
---

# CorsOptions

`CorsOptions` configures CORS for every route in an `HttpRouter`. Its required fields are `origin`, `methods`, `allowedHeaders`, `exposedHeaders`, `credentials`, and `maxAge` in seconds.

`methods` accepts only the standard uppercase HTTP method literals: `"CONNECT"`, `"DELETE"`, `"GET"`, `"HEAD"`, `"OPTIONS"`, `"PATCH"`, `"POST"`, `"PUT"`, `"QUERY"`, and `"TRACE"`. `allowedHeaders` remains `string[]` so applications can authorize custom request headers such as `"X-Request-Id"` and `"X-Api-Key"`.

When `credentials` is `true`, `origin` must list explicit origins. `"*"` is rejected by the compiler and runtime because browsers do not allow credentialed wildcard origins.

Call `app.cors(options)` before `listen(...)`. CORS is browser policy, not authentication or authorization.

<!-- @include: ../../.fragments/modules/http/cors-options/corsoptions.md -->
