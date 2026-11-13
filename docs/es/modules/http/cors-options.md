---
title: CorsOptions
description: Configuración CORS completa para un router HTTP.
---

# CorsOptions

`CorsOptions` configura CORS para cada ruta de un `HttpRouter`. Sus campos obligatorios son `origin`, `methods`, `allowedHeaders`, `exposedHeaders`, `credentials` y `maxAge` en segundos.

`methods` acepta solo los literales de métodos HTTP estándar en mayúsculas: `"CONNECT"`, `"DELETE"`, `"GET"`, `"HEAD"`, `"OPTIONS"`, `"PATCH"`, `"POST"`, `"PUT"`, `"QUERY"` y `"TRACE"`. `allowedHeaders` permanece como `string[]` para que las aplicaciones puedan autorizar headers de solicitud personalizados, como `"X-Request-Id"` y `"X-Api-Key"`.

Cuando `credentials` es `true`, `origin` debe enumerar orígenes explícitos. `"*"` se rechaza en el compilador y el runtime porque los navegadores no permiten un origen comodín con credenciales.

Llame a `app.cors(options)` antes de `listen(...)`. CORS es una política del navegador, no autenticación ni autorización.

<!-- @include: ../../../.fragments/modules/http/cors-options/corsoptions.md -->
