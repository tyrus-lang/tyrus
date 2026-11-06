---
title: CookieOptions
description: Optional security and lifetime attributes for reply cookies.
---

# CookieOptions

`CookieOptions` is the optional third argument of `reply.setCookie(name, value, options)`. It supports `domain`, `path`, `maxAge`, `secure`, `httpOnly`, and `sameSite`.

`sameSite` is the string literal union `"lax" | "strict" | "none"`. The `"none"` value requires `secure: true`. `maxAge` must be zero or positive, and `path` must begin with `/`. The runtime serializes these values as the protocol attributes `Lax`, `Strict`, and `None`.

Cookie names, values, and attributes are validated before the runtime writes the `Set-Cookie` header. Use `clearCookie(name)` to expire a cookie. The same options go in the `options` field of each `HttpCookie` passed to `reply.setCookies([...])`, and `reply.clearCookies()` expires every cookie the request sent.

<!-- @include: ../../.fragments/modules/http/cookie-options/cookieoptions.md -->
