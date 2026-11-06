---
title: CsrfOptions
description: Configure opt-in double-submit CSRF protection.
---

# CsrfOptions

`CsrfOptions` enables an opt-in double-submit CSRF check for an `HttpRouter`. `app.csrf(...)` compares a named cookie with a named request header before it reads the body or calls a handler for an unsafe request. `GET`, `HEAD`, `OPTIONS`, and `TRACE` remain readable without a token; every other method requires both values to match. A rejected request receives `403` with the stable body code `csrf_invalid`.

<SignatureBlock label="Signature">interface CsrfOptions { cookie: string; header: string; }</SignatureBlock>

<!-- @include: ../../.fragments/modules/http/csrf-options/csrfoptions.md -->

The application owns token issuance, session binding, renewal, and authorization. Generate an unpredictable token at the authentication boundary, set it in a secure host-only cookie, and send the same token in the configured header from browser code. The `__Host-` cookie prefix is a good default because browsers require it to be `Secure`, have `Path=/`, and omit `Domain`. This router policy deliberately does not protect bearer-token APIs: do not enable it for routes whose authentication does not use cookies.

::: danger error
<!-- @include: ../../.fragments/modules/http/csrf-options/csrfoptions-2.md -->
:::

Header names must be valid HTTP header names, and cookie names must be valid cookie names. A malformed policy is rejected while the server is being configured, before it can serve without the protection the source declares.
