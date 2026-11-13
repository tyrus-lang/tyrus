---
title: createRouter
description: Create an HTTP router through tyrus:http.
---

# createRouter

For listener startup and configuration, see [listen](./listen.md) and [ServerOptions](./server-options.md).

`createRouter` creates a native HTTP router. Routes are declared with a route object: `response` maps each status code to the type of its payload, and `handler` receives `(request, reply)`. A response type can be a declared interface, an array such as `User[]`, or a supported primitive such as `string`, `boolean`, or a numeric type. A route with only the default `200` response may use that type directly, while a route with multiple statuses uses a status map such as `{ 200: User[], 500: ApiError }`. Every status a handler sends must be declared in `response`, so `reply.status(...).send(...)` is validated against the schema. Request and response schemas accept scalar fields, declared nested interfaces, and arrays of either; response fields may be optional, in which case an absent field is omitted from the JSON object. Request schemas remain strict and reject properties that are not declared at any object level.

A route may answer with text instead of a JSON object by declaring `TextBody` as its response, imported from `tyrus:http` like any other type the module exports. `send` then takes a `string`, and the body is that string as it was written, with `content-type: text/plain; charset=utf-8` unless `reply.type(...)` says otherwise. The declaration is what decides, not the value: a `TextBody` route refuses an object and an object route refuses a string, so what a route answers with stays readable from the route rather than from the handler's last line. `TextBody` has no fields, because there is nothing to describe: what shapes the text is the application.

A route may answer with bytes instead of text or JSON by declaring `BytesBody`, and it may read them the same way with `body: BytesBody`. `send` then takes a `u8[]`, `request.body` is a `u8[]`, and the bytes cross the wire exactly as they are: no encoding, no replacement characters for what is not text, and `content-type: application/octet-stream` unless `reply.type(...)` names the format. `body: TextBody` is the same for text, handing the handler the body as a `string` rather than an object. Both are declared in the route rather than inferred from a content type the sender chose, and both are refused anywhere other than `body` and `response`: they describe a body, and `params`, `query`, `headers` and `cookies` are name-value pairs with no single text or byte string to be. A route reading a shapeless body accepts any content type, since there is no format for the sender to disagree with.

A route reads a `multipart/form-data` body by declaring `body: MultipartBody`, and `request.body` is then a `MultipartPart[]`. Each part carries `name`, the optional `fileName` and `contentType`, and `bytes`. The optionals are optional because absence is a real state: a plain text field of a form has no file name, and a sender is allowed not to say what a part contains. A file name is reduced to its last segment before the program sees it, so a sender cannot make one escape a directory the program chooses. A body that is not multipart, or one whose content type is missing entirely, is refused with `415`, since the boundary lives in that header; one that does not parse is `400`; and one carrying more parts, or a larger part, than the route accepts is `413`. Parts are held in memory and bounded by `bodyLimit` before a byte is read, so `MultipartBody` is a request-only type: it never appears in a `response`.

<!-- @include: ../../.fragments/modules/http/create-router/createrouter.md -->

<SignatureBlock label="Signature">createRouter(): HttpRouter</SignatureBlock>

## HttpRouter

`HttpRouter` is the native router value returned by `createRouter()`. It owns route registration and exposes the HTTP method functions (`get`, `post`, `put`, `patch`, `delete`, `head`, `options`, `connect`, `query`, and `trace`), `fallback`, `cors`, and `listen`. A `HEAD` request preserves the response status and headers but never transfers a body.

<!-- @include: ../../.fragments/modules/http/create-router/httprouter.md -->
## Route Patterns

The router supports standard HTTP methods and path parameters:

`router.query(path, options)` registers the HTTP `QUERY` method, which can carry structured search criteria in a request body. URL query-string fields remain available as `request.query`.

To group routes into named sections in Swagger UI, pass `tags` in each route's options object, for example `router.get("/users", { tags: ["user"], ... })`. Routes without tags are grouped under `default`. The same tags are written to the OpenAPI document. The generated document uses OpenAPI 3.2, so `QUERY` routes appear as operations with their declared request body.

<!-- @include: ../../.fragments/modules/http/create-router/route-patterns.md -->

## Request channels

An unexpected field is refused only in `body`, which is written by the sender for this route. The other channels always carry more than a schema names: every request has `host`, `user-agent` and `accept`, and a browser adds cookies of its own, so `params`, `query`, `headers` and `cookies` read the fields they declare and ignore the rest.

A route that declares `body` reads JSON, so a request whose `Content-Type` is another media type is refused with `415` and an `Accept: application/json` header. A vendor type that still carries JSON, such as `application/vnd.api+json`, is read. A request that declares no content type is read as before, and a route with no `body` schema does not inspect the media type at all.

A request whose body cannot be framed is refused before that body is read. Carrying both `content-length` and `transfer-encoding` gives the message two possible ends, and every hop between the client and the server is free to believe a different one, which is what request smuggling exploits; such a request is answered with `400` and `ambiguous_framing` rather than resolved by guessing. A body that is framed but unreadable, such as chunked data that does not parse, is `400` and `malformed_body`: only a body that passed the size limit is `413`.

Testing a field proves it present for the rest of that branch, so `if (request.body.nickname)` reads it as a `string` where the schema declares `string?`. This holds for any path over an immutable binding: a mutable root could be reassigned between the test and the read, so it stays optional there.

A request schema field may be optional. `nickname?: string` reads as three outcomes rather than two: present, absent, and wrong. An absent field yields the empty value, which `??` and `?.` read as usual, while a field of the wrong type is still a `400`. Optional request fields hold scalars today, because absence is told apart by the value reader and the nested and array readers have no equivalent yet; a response schema may leave out a field of any supported type, since writing one only has to omit it. Narrowing an optional field gives the value it holds, with everything that value has: inside `if (request.query.tag)` the field is a `string`, so `request.query.tag.length` and `request.query.tag.toUpperCase()` read as they would on any other string.

A schema field may be an enum. The value that crosses the wire is the value the member declares: `Role.Admin = "admin"` is the JSON string `"admin"`, and `Level.High = 9` is the number `9`. Nothing maps a number to its member name on the way out, because then the value in the program and the value on the wire would be two different things and only one of them would be what a `switch` compares against. Reading is where this earns its keep: a value of the right type that is not a declared member is refused with `400`, exactly like a value of the wrong type. Without that check the wire could hand the program a `Level` holding `5`, which is no member the program declared, and every `switch` over it would fall through. An enum whose members carry payloads is not accepted, because it has no single value to be; the refusal names the field and says why.

A response schema field may also be a discriminated union: every variant an object, with a shared field holding a distinct literal in each. It goes out as the object its live variant already is, so `{ kind: "done"; value: i32 } | { kind: "failed"; reason: string }` writes `{"kind": "done","value": 42}` and nothing else. There is no envelope, because the discriminant field is what tells the reader which variant it got. A request schema reads one back the same way, choosing the variant by its discriminant. A body whose discriminant names no declared variant is refused with `400`, exactly like a value of the wrong type: the wire cannot hand the program a variant it never declared.

Only `body` is JSON written by the sender. Path params, query, headers and cookies carry text: `?limit=25` is two characters on the wire, and `/users/123` says nothing about whether `123` is an identifier or a count. The declared schema decides how that text is read, so `id: string` accepts `/users/123` and `limit: i32` reads `?limit=25` as a number. A value the schema cannot read is rejected with `400`, naming the channel that failed, such as `Invalid request query`. A JSON body stays strict: `{"limit": "25"}` does not satisfy `limit: i32`.

<!-- @include: ../../.fragments/modules/http/request-channels.md -->

## Rejected requests

A request that does not fit its schemas is answered with `400` and a body carrying `statusCode`, `code`, `message` and `issues`. The message names the channel that failed, and each issue names the `field`, the `validator` that rejected it, and why. A request rejected before any decoration ran, such as one whose body is not JSON, has an empty `issues`.

<!-- @include: ../../.fragments/modules/http/validation-issues.md -->

## Handlers in another module

An imported controller can leave its two handler parameter types out. The route supplies the `req` shape from its request schemas and the `res` type from its declared response statuses, so the controller does not need to import those schema types or repeat the response union. The controller may still import values it uses, such as `HttpStatus`, and explicit parameter annotations remain supported. An inferred controller has one route contract; use separate controllers or an explicit shared signature when routes have different schemas.

<!-- @include: ../../.fragments/modules/http/named-handler.md -->

## Client address and trusted proxies

A route's rate limit is keyed by the client's address, and that address is the connection's peer. `app.trustedProxies([...], identityHeader?)` lists the peers whose forwarding headers the server believes, as addresses (`10.0.0.1`) or networks in CIDR form (`10.0.0.0/8`).

<!-- @include: ../../.fragments/modules/http/trusted-proxies.md -->

Behind a listed proxy the server reads the forwarding headers instead: it walks each chain from the right and skips hops that are themselves trusted, so the first address that is not is the closest one the trusted chain vouched for. `Forwarded` and `X-Forwarded-For` are both read, and both have to name the same client. A proxy usually writes one of them and does not strip the other, so letting either one decide on its own would let a caller send the spare header and name its own address, defeating the limit, the ban and the shared cap at once. When the two disagree the request counts against the peer, the only address the server observed for itself. Pass `identityHeader` when that proxy uses another address header, such as `"x-client-ip"`; it replaces the standard headers, is then the only one read, and accepts the same comma-separated address chain. It is still read only after the peer matches the trusted list, and a malformed address falls back to the peer instead of becoming a new identity. From any other peer those headers are ignored, because a client can write them, and believing them would let one caller present a new address per request and never reach a limit. One malformed entry rejects the whole list rather than silently dropping a network, and the server stops instead of starting with the wrong policy.

## Lifecycle

`app.listen(port, options?)` returns `Result<HttpServer, HttpError>` after binding. Handle a startup failure with `onfail`; its binding is the `HttpError` value. In a `void` function the failure branch returns from that function, while a successful call gives the program an `HttpServer` handle.

<!-- @include: ../../.fragments/modules/http/listen-example.md -->

`server.port()` reports the port actually bound, including when the requested port was `0`. `server.close()` stops accepting new connections and settles after requests already running finish. The runtime keeps a listener alive after `main` returns and waits for SIGINT or SIGTERM; the signal closes active servers and drains requests, bounded by `shutdownTimeout`. A second signal ends the process immediately.

`shutdownTimeout` bounds how long `close` waits. Past the deadline, connections still open are ended. Without a `shutdownTimeout`, the wait is unbounded. A close that cuts live requests reports `ERR_HTTP_SHUTDOWN_TIMEOUT`; closing an invalid handle reports `ERR_HTTP_SHUTDOWN`.

<!-- @include: ../../.fragments/modules/http/create-router/error-handling.md -->

## Route configuration

Route options may include `config`, a typed object of per-route settings. Both `config` and each of its keys are optional, but a key that is present must be complete.

<!-- @include: ../../.fragments/modules/http/route-config.md -->

`bodyLimit` is a string byte size that overrides the server default of 2 MB. Decimal units (`kb`, `mb`, `gb`) are powers of 1000 and binary units (`kib`, `mib`, `gib`) are powers of 1024, and `"off"` disables the limit entirely, which accepts any body size and is a denial-of-service tradeoff to make only for a trusted internal route. A body over the effective limit is refused with `413`.

`rateLimit` declares `max`, an integer request count per client identity, and `timeWindow`, a duration string such as `"1m"`, `"30s"`, `"500ms"` or the verbose `"10 seconds"`; both are required when `rateLimit` is present. `globalMax` is an optional integer cap shared by every identity using that limit. `ban` is an optional non-negative count of consecutive requests that exceed an identity's own limit; after that count, the identity receives `403` with `rate_limit_banned` until `banWindow` expires. `ban: 0` bans on the first exceeded request, and `banWindow` defaults to `timeWindow`. Exhaustion of `globalMax` never adds a ban strike, so a busy server cannot ban an otherwise well-behaved client. The limit is a token bucket per client identity, enforced before the request body is read. It begins with `max` tokens and refills continuously over `timeWindow`, so a request burst cannot use two full windows at their boundary. When `globalMax` is present, a second token bucket begins with that many tokens and is shared by all identities; a request proceeds only when both buckets have credit. An exhausted bucket answers `429` with `Retry-After`. The identity bucket table is bounded, and a full one evicts its least recently used identities rather than refusing a client it has never seen, so filling it from many addresses cannot turn the limiter into the denial of service it exists to prevent. An identity serving a ban is kept whatever its age: a banned request is refused before it can touch the clock, so it is the stalest record in the table and would otherwise be the first thing evicted, ending the ban exactly when the pressure that earned it arrived.

A route's `rateLimit` replaces `app.rateLimit(...)` for that route rather than adding to it, so a route can be looser as easily as stricter than the server-wide default. Setting the limit on the router is the usual place for it: a limit that has to be repeated on every route is one some route will end up missing.

The `bodyLimit` and `timeWindow` formats are validated during compilation, so they are written inline.

## CORS

`app.cors(options)` installs the CORS middleware over the whole router, and handles preflight `OPTIONS` requests before route dispatch. `origin`, `allowedHeaders` and `exposedHeaders` are string arrays; `methods` is restricted to the uppercase HTTP method literals, so a misspelling is a type error. `allowedHeaders` stays a plain string array on purpose, because an application may authorize a header it invented. `maxAge` is in seconds, and `0` leaves it unset.

## CSRF

`app.csrf({ cookie, header })` enables an opt-in double-submit check for cookie-authenticated routes. Before an unsafe request reaches a handler, the runtime compares the configured cookie and header in constant time; a missing or different value returns `403` with `csrf_invalid`. Token generation and session binding remain application policy. See [CsrfOptions](./csrf-options.md).

`"*"` is accepted in `origin` and `allowedHeaders`, but not beside `credentials: true`. A browser refuses that pair outright rather than narrowing it, so every credentialed request would fail; the compiler reports it at the `"*"` that caused it.

::: danger error
<!-- @include: ../../.fragments/modules/http/cors-credentials.md -->
:::

Origins are checked while they are still literals. A browser sends `Origin: <scheme>://<host>[:<port>]` and nothing else, so an entry written with a path, a query, a trailing slash or no scheme at all is compared against a value that can never contain one, and silently matches nothing. That failure is invisible at run time, which is why it is a compile error here. An entry that carries a path gets a Quick Fix that cuts it; a missing scheme does not, because `http` and `https` are different origins and a guess would produce a rule that looks right and matches nothing.

An empty `origin` list is refused too: it matches no origin at all, which refuses every cross-origin request, and a router that wanted that would simply not call `cors`.

The same rules are enforced again at startup for values that are not literals. Router registration methods `get`, `post`, `put`, `patch`, `delete`, `head`, `options`, `connect`, `query`, `trace`, `fallback`, `ws` and `docs`, plus `onResponse`, `cors`, `csrf`, `fetchPolicy`, `rateLimit` and `trustedProxies`, return `Result<void, HttpError>` when the runtime refuses a route or configuration. Setup can propagate these failures with `?` or handle them with `match` before listening.

## Outgoing requests

`app.fetchPolicy(options)` says where `fetch` is allowed to go. Nothing is refused until a program writes it: talking to `10.0.0.5` or to `localhost` is ordinary, and a runtime that refused it by default would teach its author to switch the protection off entirely.

<!-- @include: ../../.fragments/modules/http/fetch-policy.md -->

`denyPrivate` refuses destinations that are only reachable from inside: loopback, the private ranges, link-local, and the rest of the space no host on the public internet answers on. `allow` is an exemption list, not an allowlist. It names hosts that may be reached even though their address would be refused, so adding an entry can only widen what a program reaches, never narrow it.

The policy exists for one shape of program: a route that hands `fetch` a URL the client chose. The request then leaves from inside the network, so it reaches whatever the network was protecting. `169.254.169.254` is the cloud metadata service, which answers credentials to anyone who can reach it and treats reachability as the authentication; a cache or an admin panel on a private address is protected the same way.

A destination is judged by the address it resolves to, not by how it is spelled. Checking the host name would not be a check at all: nothing stops a public name from resolving to a private address, and a policy that trusted the spelling would approve the metadata service the moment someone pointed a domain at it. For the same reason, every hop of a redirect is checked like the first one, because a redirect names a destination the program never wrote.

A refused destination rejects the promise with `ERR_HTTP_FETCH_BLOCKED`, which is its own code rather than the one an unresolvable name produces: a caller told their host name is wrong would go looking in the wrong place for something their own policy decided.

## Security headers

Security headers are written the way every other response header is, with `reply.header(name, value)`. There is no preset that adds them, so what a browser receives is what the handler wrote, and the editor offers all four by name, with their values, inside `reply.header("`.

<!-- @include: ../../.fragments/modules/http/security-headers.md -->

<!-- @include: ../../.fragments/modules/http/security-headers-table.md -->

Each one is a narrow instruction to a browser, and a client that ignores it is unaffected.

The editor also offers `content-security-policy` and `strict-transport-security`, and leaves the value of both to the application. A policy generic enough for every application is either so loose it protects nothing or so strict it breaks the page, so what it should say is the application's to decide. `strict-transport-security` is a promise about TLS: send it only from behind TLS, because over plain HTTP it asks a browser to refuse the connection that works.

None of this makes an application safe from cross-site scripting. These headers reduce what a browser does with a response; they say nothing about what the application puts in one.

A response the router produced on its own, such as a `404` for an unmatched path, ran no handler and so carries none of them.

## Responses that do not match their schema

When a handler produces a response that does not satisfy the schema its route declared, the client receives a generic `500` carrying `code: "internal_server_error"` and nothing else. The client did not cause the failure and cannot act on it, and the field names involved describe the server's own schema rather than anything the client sent.

The field and the rule that failed go to stderr instead, where the operator who can fix it will see them. This is deliberately not the `400` contract: a rejected request does name the field and the rule in its body, because there the failure is the client's to correct.

## Pre-handlers

Route options may include `preHandler`, an array literal of named synchronous functions. Import `HttpRequest` and `HttpResponse` when declaring a pre-handler. It receives the raw `HttpRequest` and returns `HttpResponse | true`: `true` continues to the next pre-handler or controller, while an `HttpResponse` stops the chain and is sent immediately. The chain is fixed at compile time and is available on both routes and `fallback`.

For OpenAPI generation and Swagger UI configuration, including authentication, custom styles and protected documentation routes, see [OpenAPI / Swagger](./openapi-swagger.md).

## Error Handling

Declare the error body in the application, then map a domain error at the controller boundary to the selected HTTP status. The HTTP status is available to clients as the response status; it does not need to be duplicated in JSON.

A handler cannot let an error escape: the server calls it with no `catch` around it, so a handler that throws, inline or referenced by name, is refused with `E3298`. Catch the error inside the handler and answer with the status it maps to.

<!-- @include: ../../.fragments/modules/http/create-router/error-handling.md -->
