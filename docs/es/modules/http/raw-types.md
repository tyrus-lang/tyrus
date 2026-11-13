---
title: Tipos HTTP sin procesar
description: Use los contratos sin procesar de solicitud, respuesta y CORS de tyrus:http.
---

# Tipos HTTP sin procesar

`HttpRequest`, `HttpResponse`, `CorsOptions` y `CookieOptions` son interfaces explícitas exportadas por `tyrus:http`. Impórtelas solo al trabajar en el límite HTTP sin procesar. Los controllers normalmente deben usar la solicitud inferida por el schema de la ruta y enviar mediante `HttpReply`.

`HttpRequest` expone un `id` inmutable, el método de solicitud, la ruta y los canales string sin procesar `body`, `params`, `query`, `headers` y `cookies`. El runtime asigna `id` una vez por solicitud y envía el valor Base64URL compacto sin padding en la cabecera de respuesta `x-request-id`. `{ logger: true }` escribe un evento de acceso estructurado en stderr con el ID, método, ruta, estado y duración; está desactivado por defecto. El texto de query, headers, cookies, credenciales y cuerpos nunca aparecen en ese evento. Use [HttpAccessLog](./http-access-log.md) con [`app.onResponse`](./on-response.md) cuando el código de la aplicación necesite esos metadatos. Consulte [HttpResponse](./http-response.md), [CorsOptions](./cors-options.md) y [CookieOptions](./cookie-options.md) para los contratos dedicados.

<!-- @include: ../../../.fragments/modules/http/raw-types/raw-http-types.md -->
