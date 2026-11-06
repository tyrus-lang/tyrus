---
title: CsrfOptions
description: Configura la protección CSRF double-submit opcional.
---

# CsrfOptions

`CsrfOptions` habilita una comprobación CSRF double-submit opcional para un `HttpRouter`. `app.csrf(...)` compara una cookie nombrada con una cabecera de petición nombrada antes de leer el cuerpo o llamar a un handler para una petición insegura. `GET`, `HEAD`, `OPTIONS` y `TRACE` siguen siendo legibles sin token; cualquier otro método exige que ambos valores coincidan. Una petición rechazada recibe `403` con el código estable `csrf_invalid` en el cuerpo.

<SignatureBlock label="Firma">interface CsrfOptions { cookie: string; header: string; }</SignatureBlock>

<!-- @include: ../../../.fragments/modules/http/csrf-options/csrfoptions.md -->

La aplicación es responsable de emitir el token, asociarlo a la sesión, renovarlo y autorizarlo. Genera un token impredecible en la frontera de autenticación, colócalo en una cookie segura host-only y envía el mismo token en la cabecera configurada desde el código del navegador. El prefijo de cookie `__Host-` es un buen valor predeterminado porque los navegadores exigen que sea `Secure`, tenga `Path=/` y omita `Domain`. Esta política del router deliberadamente no protege APIs bearer-token: no la habilites para rutas cuya autenticación no use cookies.

::: danger error
<!-- @include: ../../../.fragments/modules/http/csrf-options/csrfoptions-2.md -->
:::

Los nombres de cabecera deben ser nombres HTTP válidos y los nombres de cookie deben ser nombres de cookie válidos. Una política malformada se rechaza mientras se configura el servidor, antes de que pueda servir sin la protección declarada en el código.
