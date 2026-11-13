---
title: HttpReply
description: API de reply HTTP tipada para controllers tyrus.
---

# HttpReply

`HttpReply<T>` es el objeto de response tipado usado por controllers HTTP.

Declara los cuerpos de error de la aplicación con los campos obligatorios `code: string` y `message: string`. El handler debe enviar ambos campos, por ejemplo `{ code: "user_not_found", message: "User not found" }`; Tyrus no deriva ni sobrescribe `code` a partir del estado HTTP. OpenAPI documenta los campos declarados por la aplicación.

Los controllers pueden retornar `void` o `Promise<void>`. Los controllers async inline infieren `Promise<void>` cuando se omite la anotación de retorno, y el runtime HTTP espera su finalización antes de cerrar la respuesta. Las verificaciones de envío único y de todos los caminos exitosos incluyen código a ambos lados de `await`.

## Uso

<!-- @include: ../../../.fragments/modules/http/http-reply/usage.md -->
Los schemas de respuesta de error pertenecen a la aplicación. Un service debe devolver datos de dominio o lanzar un error de dominio; un controller o un futuro límite de errores lo mapea a un status HTTP y a este cuerpo de respuesta declarado.

## Headers y cookies

Prepare los metadatos de la response antes de `status(...).send(...)`. `header(name, value)` sustituye un header, `appendHeader(name, value)` agrega otro valor, `removeHeader(name)` elimina valores definidos antes en el reply y `type(value)` define `Content-Type`.

<!-- @include: ../../../.fragments/modules/http/http-reply/headers-and-cookies.md -->

Los nombres y valores de headers se validan en el límite nativo. Los headers que controlan el framing del mensaje (`connection`, `content-length` y `transfer-encoding`) y `set-cookie` no se pueden cambiar mediante la API genérica de headers. Use `setCookie(name, value, options?)` y `clearCookie` para cookies. `setCookies(cookies)` define de una vez una lista de valores `HttpCookie` (`name`, `value`, `options?`) y define todos o ninguno: cuando se rechaza cualquier entrada, no se escribe ninguna cookie de la lista. `clearCookies()` expira todas las cookies que envió el request; el servidor solo conoce esas, así que un request sin cookies deja la respuesta sin cambios. Las opciones de cookie pueden definir `domain`, `path`, `maxAge`, `secure`, `httpOnly` y `sameSite`; `sameSite: "none"` requiere `secure: true`. JSON es el tipo de contenido predeterminado cuando no se llama a `type(...)`.

Un `send` o `stream` debe ir precedido por `status(...)`: el slot de respuesta empieza en `500`, así que enviar sin elegir un estado serviría el cuerpo de éxito bajo un estado de fallo. El verificador lo reporta en el método final de la respuesta. `stream()` solo está disponible para un estado declarado como `StreamBody`; devuelve un `WritableStream<u8[]>` que inicia la respuesta progresivamente. Consulta [`StreamBody`](./stream-body.md) para la firma y el ciclo de vida.

Los nombres y valores de cabeceras y cookies se validan. Un nombre escrito como literal se comprueba en tiempo de compilación, con el error apuntando al literal; un nombre construido en tiempo de ejecución se comprueba en la frontera nativa, donde el rechazo se escribe en stderr y la cabecera o cookie no se envía. Las cabeceras de framing (`connection`, `content-length`, `transfer-encoding`) y `set-cookie` están protegidas: usa los métodos de cookie para las cookies.

## Caché condicional

Pase metadatos de caché como el segundo argumento opcional de `send`. `HttpCacheOptions.etag` es el valor opaco del validador; Tyrus lo entrecomilla y lo valida. Establezca `weak: true` solo cuando el validador sea intencionalmente débil. `lastModified` acepta un `Instant` y se formatea como fecha HTTP, mientras que `control` se convierte en `Cache-Control`. Cuando `If-None-Match` contiene la misma etiqueta, incluida su forma débil o una etiqueta en una lista separada por comas, Tyrus devuelve `304 Not Modified`, conserva el ETag y omite el cuerpo.

Use una fecha HTTP válida en `last-modified` cuando la representación tenga un instante de modificación significativo. En ausencia de `If-None-Match`, `If-Modified-Since` igual o posterior a ese instante también produce `304 Not Modified`. `If-None-Match` siempre tiene precedencia.

## Compresión de respuesta

Tyrus negocia `br` o `gzip` automáticamente para respuestas almacenadas en búfer, comprimibles y de al menos 1024 bytes cuando la solicitud los acepta. Brotli gana cuando ambos tienen la misma calidad. Una respuesta comprimida incluye su `Content-Encoding` y `Vary: Accept-Encoding`. Nunca recomprime un `Content-Encoding` explícito, formatos ya comprimidos ni una respuesta con ETag fuerte, porque un validador fuerte identifica una representación exacta. La compresión por streaming se planifica por separado.

<!-- @include: ../../../.fragments/modules/http/http-reply/response-compression.md -->

<!-- @include: ../../../.fragments/modules/http/http-reply/response-compression-2.md -->

`controller-must-send-response` valida que caminos exitosos del controller envíen una respuesta.
