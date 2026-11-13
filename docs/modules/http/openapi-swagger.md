---
title: OpenAPI / Swagger
description: Generate and configure API documentation with OpenAPI and Swagger UI.
---

# OpenAPI / Swagger

`app.docs()` belongs to `tyrus:http`. It builds an OpenAPI 3.2 document from the routes registered on the router and serves a Swagger UI page from the application itself. No external documentation service is required.

## Enable the documentation

Call `app.docs(options)` on the router. The default page is `/docs`, and the OpenAPI document is available at `/docs/openapi.json`. Set `path` to use another base path. Routes registered before or after this call are included.

`app.docs(options)` returns `Result<void, HttpError>` because the runtime can reject an invalid documentation configuration. Propagate the result with `?` during router setup or handle it with `match`.

| Option | Type | Default | Purpose |
| --- | --- | --- | --- |
| `path` | `string` | `"/docs"` | Base path for the Swagger UI, OpenAPI document and bundled assets. It must start with `/`. |
| `title` | `string` | `"API"` | API title shown by Swagger UI and in the OpenAPI `info` object. |
| `version` | `string` | `"1.0.0"` | API version written to the OpenAPI `info` object. |
| `description` | `string` | empty | API description written to the OpenAPI `info` object. |
| `auth` | object literal | none | Security schemes shown by Swagger UI and declared by the OpenAPI document. It does not authenticate requests by itself. |
| `css` | `string` | empty | CSS source inserted into the page after the bundled Swagger UI stylesheet. Pass CSS text, not a file path. |
| `preHandler` | named function array | none | Synchronous guards run before the page, OpenAPI document and Swagger UI assets are served. |

The options object is checked inline by the compiler. There is no named options interface to import. These are all options currently accepted by `app.docs`.

## What the document contains

The document describes each route's method and path, path parameters, declared `params`, `query`, `headers` and `cookies`, request bodies, response statuses and response schemas. Interfaces and enums become reusable named schemas. JSON request bodies include generated examples that satisfy their declared schema constraints. `Instant` is described as a date-time string.

Add `tags` to route options to group operations under named sections in Swagger UI. Routes without tags appear under `default`.

```tyrus
router.get("/users", {
    tags: ["users"],
    response: { 200: User[] },
    handler: listUsers,
});
```

The HTTP `QUERY` method is represented as an OpenAPI 3.2 operation, including its declared request body. WebSocket routes appear as `GET` operations with a `101 Switching Protocols` response and the `x-tyrus-websocket` extension. The document is assembled from route declarations, so a route does not need a separate OpenAPI definition.

## Configure Swagger UI authorization

Use `auth` to describe how a client authorizes in the Swagger UI. The schemes appear in the **Authorize** control and are written to the OpenAPI security definitions. The scheme description is not an access check: protect API routes with their own authentication and authorization logic.

### Bearer token

```tyrus
auth: { bearer: true, bearerFormat: "JWT" }
```

`bearerFormat` is optional and can only be used with `bearer: true`.

### API key

```tyrus
auth: { apiKey: { name: "x-api-key", in: "header" } }
```

`name` is the key's header, query parameter or cookie name. `in` must be `"header"`, `"query"` or `"cookie"`; prefer a header for credentials.

## Customize the page with CSS

`css` takes CSS source, not a path to a stylesheet. The runtime places it in a `<style>` element after Swagger UI's own stylesheet, so these declarations can override its defaults.

```tyrus
css: ".swagger-ui .topbar { background: #16324f; } .swagger-ui .info { margin: 2rem 0; }"
```

## Protect the documentation endpoints

`preHandler` receives the raw `HttpRequest` and must be a literal array of named synchronous functions with the signature `(request: HttpRequest) => HttpResponse | true`. Return `true` to serve the requested documentation resource. Return an `HttpResponse` to stop the request, for example with `HttpStatus.Forbidden`. The same guard protects the HTML page, `/openapi.json`, the stylesheet and the script bundle.

Use the application's real session or credential validator inside the guard. This example assumes `isAuthorizedAdmin` is an existing application function; it is not provided by `tyrus:http`.

<!-- @include: ../../.fragments/modules/http/openapi-swagger/pre-handler-example.md -->

The `auth` setting above only tells Swagger UI how to send credentials when trying an operation. It does not protect the documentation page, does not validate credentials, and does not replace route-level authorization. Use `preHandler` to gate access to the documentation itself and the application's normal authorization checks to secure API operations.

## Complete setup example

This example combines the API metadata, a bearer-token scheme, custom CSS and the documentation guard. Replace the example title and authorization scheme with those used by the application.

<!-- @include: ../../.fragments/modules/http/openapi-swagger/setup-example.md -->
