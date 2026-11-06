---
title: HttpResponse
description: Response HTTP sin procesar usada por pre-handlers síncronos.
---

# HttpResponse

`HttpResponse` es la response sin procesar devuelta por un `preHandler` síncrono cuando debe detener la solicitud antes de que se ejecute el controller. Lleva una respuesta HTTP anticipada completa; los controllers normales deben usar `HttpReply`.

Contiene `statusCode: HttpStatus`, `body: string | u8[]`, `contentType: string` opcional y `cookies: string`. El cuerpo se envía tal como se proporciona: el texto no se serializa como JSON y los bytes admiten archivos u otros payloads binarios. Sin `contentType`, el texto usa `text/plain; charset=utf-8` y los bytes usan `application/octet-stream`. Defina `cookies` como string vacía cuando el pre-handler no necesite enviar cookies.

Importe `HttpResponse` explícitamente junto con `HttpRequest` al declarar un pre-handler.

<!-- @include: ../../../.fragments/modules/http/http-response/httpresponse.md -->
