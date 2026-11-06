---
title: listen
description: Start an HTTP server and handle startup errors.
---

# listen

`app.listen(port, options?)` binds the router and returns `Result<HttpServer, HttpError>`. The server starts accepting requests immediately, and `main` can continue running setup code. Use `onfail` to handle a bind or TLS startup error; in a `void` function the failure branch returns from that function.

<SignatureBlock label="Signature">app.listen(port: i32, options?: ServerOptions): Result&lt;HttpServer, HttpError&gt;</SignatureBlock>

## Print the bound port

`listen` returns the bound server handle on success, so `server.port()` reports the actual port, including when the requested port was `0`. The runtime keeps the listener active after `main` returns and waits for an operating-system shutdown signal; call `server.close()` when the application needs to stop it earlier.

<!-- @include: ../../.fragments/modules/http/listen-example.md -->

## Handle startup failure

The `onfail` binding contains `HttpError`, with a stable `code` and a readable `message`. It handles the error without throwing; the branch returns from this `void` function after reporting it.

<!-- @include: ../../.fragments/modules/http/listen-catch.md -->

## Options

Pass a `ServerOptions` object as the second argument to `listen`. See [ServerOptions](./server-options.md) for the available settings.
