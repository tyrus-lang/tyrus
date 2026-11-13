---
title: HttpAccessLog
description: Metadados imutáveis de requisição e resposta para hooks de resposta de tyrus:http.
---

# HttpAccessLog

`HttpAccessLog` é o evento imutável recebido por um callback `app.onResponse(...)`. Ele contém apenas metadados de roteamento e resposta. Nunca inclui string de query, cabeçalhos, cookies, credenciais ou conteúdo do corpo.

## Assinatura

<!-- @include: ../../../.fragments/modules/http/http-access-log/signature.md -->

`requestId` também é enviado ao cliente no cabeçalho de resposta `x-request-id`. `duration` é o tempo decorrido no servidor até o corpo da resposta terminar ou ser descartado, representado como um `Duration` de `tyrus:temporal`.

<!-- @include: ../../../.fragments/modules/http/http-access-log/signature-2.md -->
