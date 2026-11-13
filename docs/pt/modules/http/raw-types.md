---
title: Tipos HTTP brutos
description: Use os contratos brutos de requisição, resposta e CORS de tyrus:http.
---

# Tipos HTTP brutos

`HttpRequest`, `HttpResponse`, `CorsOptions` e `CookieOptions` são interfaces explícitas exportadas por `tyrus:http`. Importe-as somente ao trabalhar na fronteira HTTP bruta. Controllers normalmente devem usar a requisição inferida pelo schema da rota e enviar por `HttpReply`.

`HttpRequest` expõe `id` imutável, o método, o caminho e os canais string brutos `body`, `params`, `query`, `headers` e `cookies`. O runtime atribui `id` uma vez por requisição e envia o valor Base64URL compacto sem padding no header de resposta `x-request-id`. `{ logger: true }` escreve um evento de acesso estruturado em stderr com o ID, método, caminho, status e duração; ele vem desligado por padrão. Texto da query, headers, cookies, credenciais e corpos nunca entram nesse evento. Use [HttpAccessLog](./http-access-log.md) com [`app.onResponse`](./on-response.md) quando o código da aplicação precisar desses metadados. Veja [HttpResponse](./http-response.md), [CorsOptions](./cors-options.md) e [CookieOptions](./cookie-options.md) para os contratos dedicados.

<!-- @include: ../../../.fragments/modules/http/raw-types/raw-http-types.md -->
