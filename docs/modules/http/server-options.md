---
title: ServerOptions
description: Configure HTTP server binding, limits, shutdown, logging, and TLS.
---

# ServerOptions

`ServerOptions` configures the listener passed to `app.listen(...)`. Pass the options as an object, either inline or through a value typed as `ServerOptions`.

<SignatureBlock label="Signature">interface ServerOptions</SignatureBlock>

## Settings

| Setting | Purpose |
|---------|---------|
| `host` | IP address to bind. Defaults to all interfaces. |
| `requestTimeout` | Maximum time for a request, including its body and handler. Expired requests receive `503`. |
| `maxConcurrentRequests` | Maximum number of requests running at once. Additional requests wait for a permit and remain bounded by `requestTimeout`. |
| `maxConnections` | Maximum number of open connections. It is unset by default; when reached, the server stops accepting connections. |
| `shutdownTimeout` | Maximum time to wait for active requests during shutdown. |
| `logger` | Writes one structured access event per completed response to stderr when `true`; defaults to `false`. |
| `headerTimeout` | Maximum time to receive request headers. Defaults to 10 seconds and cannot be disabled. |
| `maxHeaderSize` | Maximum combined request-line and header size. Defaults to 64 KiB and has a minimum of 8 KiB. |
| `keepAlive` | Allows a connection to carry multiple requests. Enabled by default. |
| `tls` | Serves HTTPS with the configured certificate and private key. Both are required. |

`rateLimit` is configured on the router with `app.rateLimit(...)`, not in `ServerOptions`. Header limits apply before a route runs, while `requestTimeout` covers a request after its headers arrive.

## Example

<!-- @include: ../../.fragments/modules/http/server-options-example.md -->
