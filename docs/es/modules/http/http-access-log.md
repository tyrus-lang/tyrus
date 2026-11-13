---
title: HttpAccessLog
description: Metadatos inmutables de solicitud y respuesta para hooks de respuesta de tyrus:http.
---

# HttpAccessLog

`HttpAccessLog` es el evento inmutable recibido por un callback `app.onResponse(...)`. Contiene solo metadatos de enrutamiento y respuesta. Nunca incluye query string, cabeceras, cookies, credenciales ni contenido del cuerpo.

## Firma

<!-- @include: ../../../.fragments/modules/http/http-access-log/signature.md -->

`requestId` también se envía al cliente en la cabecera de respuesta `x-request-id`. `duration` es el tiempo transcurrido en el servidor hasta que el cuerpo de respuesta termina o se descarta, representado como un `Duration` de `tyrus:temporal`.

<!-- @include: ../../../.fragments/modules/http/http-access-log/signature-2.md -->
