---
title: CookieOptions
description: Atributos opcionales de seguridad y duración para cookies de reply.
---

# CookieOptions

`CookieOptions` es el tercer argumento opcional de `reply.setCookie(name, value, options)`. Admite `domain`, `path`, `maxAge`, `secure`, `httpOnly` y `sameSite`.

`sameSite` es la unión de literales de string `"lax" | "strict" | "none"`. El valor `"none"` requiere `secure: true`. `maxAge` debe ser cero o positivo y `path` debe comenzar con `/`. El runtime serializa estos valores como los atributos de protocolo `Lax`, `Strict` y `None`.

Los nombres, valores y atributos de cookies se validan antes de que el runtime escriba el header `Set-Cookie`. Use `clearCookie(name)` para expirar una cookie. Las mismas opciones van en el campo `options` de cada `HttpCookie` pasado a `reply.setCookies([...])`, y `reply.clearCookies()` expira todas las cookies que envió el request.

<!-- @include: ../../../.fragments/modules/http/cookie-options/cookieoptions.md -->
