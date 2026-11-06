---
title: HttpResponse
description: Resposta HTTP bruta usada por pré-handlers síncronos.
---

# HttpResponse

`HttpResponse` é a resposta bruta retornada por um `preHandler` síncrono quando ele precisa interromper a requisição antes de o controller executar. Ele carrega uma resposta HTTP antecipada completa; controllers normais devem usar `HttpReply`.

Ele contém `statusCode: HttpStatus`, `body: string | u8[]`, `contentType: string` opcional e `cookies: string`. O corpo é enviado como fornecido: texto não é serializado como JSON e bytes aceitam arquivos ou outros payloads binários. Sem `contentType`, texto usa `text/plain; charset=utf-8` e bytes usam `application/octet-stream`. Defina `cookies` como string vazia quando o pré-handler não precisar enviar cookies.

Importe `HttpResponse` explicitamente junto de `HttpRequest` ao declarar um pré-handler.

<!-- @include: ../../../.fragments/modules/http/http-response/httpresponse.md -->
