---
title: CorsOptions
description: Configuração CORS completa para um roteador HTTP.
---

# CorsOptions

`CorsOptions` configura CORS para cada rota de um `HttpRouter`. Seus campos obrigatórios são `origin`, `methods`, `allowedHeaders`, `exposedHeaders`, `credentials` e `maxAge` em segundos.

`methods` aceita somente os literais de métodos HTTP padrão em maiúsculas: `"CONNECT"`, `"DELETE"`, `"GET"`, `"HEAD"`, `"OPTIONS"`, `"PATCH"`, `"POST"`, `"PUT"`, `"QUERY"` e `"TRACE"`. `allowedHeaders` permanece `string[]` para que aplicações possam autorizar headers de requisição personalizados, como `"X-Request-Id"` e `"X-Api-Key"`.

Quando `credentials` é `true`, `origin` deve listar origens explícitas. `"*"` é rejeitado pelo compilador e pelo runtime porque navegadores não permitem origem curinga com credenciais.

Chame `app.cors(options)` antes de `listen(...)`. CORS é uma política do navegador, não autenticação nem autorização.

<!-- @include: ../../../.fragments/modules/http/cors-options/corsoptions.md -->
