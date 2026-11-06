---
title: CookieOptions
description: Atributos opcionais de segurança e duração para cookies de reply.
---

# CookieOptions

`CookieOptions` é o terceiro argumento opcional de `reply.setCookie(name, value, options)`. Ele suporta `domain`, `path`, `maxAge`, `secure`, `httpOnly` e `sameSite`.

`sameSite` é a união de literais de string `"lax" | "strict" | "none"`. O valor `"none"` exige `secure: true`. `maxAge` deve ser zero ou positivo e `path` deve começar com `/`. O runtime serializa esses valores como os atributos de protocolo `Lax`, `Strict` e `None`.

Nomes, valores e atributos de cookies são validados antes de o runtime escrever o header `Set-Cookie`. Use `clearCookie(name)` para expirar um cookie. As mesmas opções vão no campo `options` de cada `HttpCookie` passado para `reply.setCookies([...])`, e `reply.clearCookies()` expira todos os cookies que o request enviou.

<!-- @include: ../../../.fragments/modules/http/cookie-options/cookieoptions.md -->
