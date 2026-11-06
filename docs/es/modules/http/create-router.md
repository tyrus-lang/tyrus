---
title: createRouter
description: Crea un router HTTP mediante tyrus:http.
---

# createRouter

Para iniciar el servidor y configurar el listener, consulte [listen](./listen.md) y [ServerOptions](./server-options.md).

`createRouter` crea un router HTTP nativo. Las rutas se declaran con un objeto de ruta: `response` mapea cada status code al tipo de su payload, y `handler` recibe `(request, reply)`. El tipo de respuesta puede ser una interface declarada, un array como `User[]` o un primitivo admitido, como `string`, `boolean` o un tipo numérico. Una ruta con solo la respuesta predeterminada `200` puede usar el tipo directamente; una ruta con varios status usa un mapa, como `{ 200: User[], 500: ApiError }`. Cada status que envía un handler debe estar declarado en `response`, así que `reply.status(...).send(...)` se valida contra el schema. Los schemas de solicitud y respuesta aceptan campos escalares, interfaces anidadas declaradas y arrays de ambos; los campos de respuesta pueden ser opcionales y, en ese caso, un campo ausente se omite del objeto JSON. Los schemas de solicitud siguen siendo estrictos y rechazan propiedades que no están declaradas en cualquier nivel del objeto.

Una ruta puede responder con texto en vez de un objeto JSON declarando `TextBody` como su respuesta, importado de `tyrus:http` como cualquier otro tipo que el módulo exporta. `send` pasa a recibir un `string`, y el cuerpo es esa cadena tal como se escribió, con `content-type: text/plain; charset=utf-8` salvo que `reply.type(...)` diga otra cosa. Quien decide es la declaración, no el valor: una ruta `TextBody` rechaza un objeto y una ruta de objeto rechaza una cadena, así que lo que responde una ruta sigue siendo legible en la ruta y no en la última línea del handler. `TextBody` no tiene campos, porque no hay nada que describir: quien da forma al texto es la aplicación.

Una ruta puede responder con bytes en vez de texto o JSON declarando `BytesBody`, y puede leerlos del mismo modo con `body: BytesBody`. `send` pasa a recibir un `u8[]`, `request.body` es un `u8[]`, y los bytes cruzan el cable exactamente como son: sin codificación, sin caracteres de reemplazo para lo que no es texto, y con `content-type: application/octet-stream` salvo que `reply.type(...)` nombre el formato. `body: TextBody` es el equivalente para texto, entregando al handler el cuerpo como un `string` en vez de un objeto. Ambos se declaran en la ruta y no se infieren de un content type que el remitente eligió, y ambos se rechazan en cualquier lugar que no sea `body` y `response`: describen un cuerpo, y `params`, `query`, `headers` y `cookies` son pares nombre-valor sin una única cadena de texto o de bytes que ser. Una ruta que lee un cuerpo sin forma acepta cualquier content type, ya que no hay formato sobre el cual el remitente pueda discrepar.

Una ruta lee un cuerpo `multipart/form-data` declarando `body: MultipartBody`, y `request.body` pasa a ser un `MultipartPart[]`. Cada parte lleva `name`, los opcionales `fileName` y `contentType`, y `bytes`. Los opcionales son opcionales porque la ausencia es un estado real: un campo de texto de un formulario no tiene nombre de archivo, y el remitente puede no decir qué contiene la parte. El nombre de archivo se reduce a su último segmento antes de que el programa lo vea, así que un remitente no puede hacer que escape de un directorio que el programa elige. Un cuerpo que no es multipart, o cuyo content type falta, se rechaza con `415`, ya que el boundary vive en esa cabecera; uno que no parsea es `400`; y uno que trae más partes, o una parte mayor, de las que la ruta acepta es `413`. Las partes se mantienen en memoria y están limitadas por `bodyLimit` antes de leer un byte, así que `MultipartBody` es un tipo solo de petición: nunca aparece en un `response`.

<!-- @include: ../../../.fragments/modules/http/create-router/createrouter.md -->

<SignatureBlock label="Firma">createRouter(): HttpRouter</SignatureBlock>

## HttpRouter

`HttpRouter` es el valor nativo de router devuelto por `createRouter()`. Registra rutas y expone las funciones de método HTTP (`get`, `post`, `put`, `patch`, `delete`, `head`, `options`, `connect`, `query` y `trace`), `fallback`, `cors` y `listen`. Una solicitud `HEAD` conserva el estado y los headers de la respuesta, pero nunca transfiere un cuerpo.

<!-- @include: ../../../.fragments/modules/http/create-router/httprouter.md -->
## Patrones de ruta

El router soporta métodos HTTP estándar y parámetros de ruta:

`router.query(path, options)` registra el método HTTP `QUERY`, que puede llevar criterios de búsqueda estructurados en el cuerpo de la solicitud. Los campos de la query string de la URL siguen disponibles en `request.query`.

Para agrupar rutas en secciones con nombre en Swagger UI, pase `tags` en el objeto de opciones de cada ruta, por ejemplo `router.get("/users", { tags: ["user"], ... })`. Las rutas sin tags se agrupan en `default`. Las mismas tags se escriben en el documento OpenAPI. El documento generado usa OpenAPI 3.2, por lo que las rutas `QUERY` aparecen como operaciones con el cuerpo de solicitud declarado.

<!-- @include: ../../../.fragments/modules/http/create-router/route-patterns.md -->

## Canales de la petición

Un campo inesperado se rechaza solo en el `body`, que lo escribe quien envía para esta ruta. Los otros canales siempre llevan más de lo que un esquema nombra: toda petición tiene `host`, `user-agent` y `accept`, y un navegador añade cookies propias, así que `params`, `query`, `headers` y `cookies` leen los campos que declaran e ignoran el resto.

Una ruta que declara `body` lee JSON, así que una petición cuyo `Content-Type` es otro media type se rechaza con `415` y una cabecera `Accept: application/json`. Un tipo de proveedor que aún lleva JSON, como `application/vnd.api+json`, sí se lee. Una petición que no declara content type se lee como antes, y una ruta sin esquema de `body` no inspecciona el media type.

Una petición cuyo cuerpo no puede delimitarse se rechaza antes de leer ese cuerpo. Llevar `content-length` y `transfer-encoding` a la vez da al mensaje dos finales posibles, y cada salto entre el cliente y el servidor es libre de creer uno distinto, que es lo que explota el request smuggling; una petición así se responde con `400` y `ambiguous_framing`, en vez de resolverse adivinando. Un cuerpo delimitado pero ilegible, como datos chunked que no se pueden analizar, es `400` y `malformed_body`: solo un cuerpo que superó el límite de tamaño es `413`.

Probar un campo demuestra que está presente en el resto de esa rama, así que `if (request.body.nickname)` lo lee como `string` donde el esquema declara `string?`. Esto vale para cualquier ruta sobre un binding inmutable: una raíz mutable podría reasignarse entre la prueba y la lectura, así que allí el campo sigue siendo opcional.

Un campo de esquema de petición puede ser opcional. `nickname?: string` se lee como tres resultados en vez de dos: presente, ausente y erróneo. Un campo ausente produce el valor vacío, que `??` y `?.` leen con normalidad, mientras que un campo de tipo equivocado sigue siendo `400`. Los campos opcionales de petición hoy guardan escalares, porque la ausencia la distingue el lector de valor y los lectores de objeto anidado y de array aún no tienen equivalente; un esquema de respuesta puede omitir un campo de cualquier tipo soportado, ya que escribir uno solo tiene que dejarlo fuera. Estrechar un campo opcional entrega el valor que guarda, con todo lo que ese valor tiene: dentro de `if (request.query.tag)` el campo es un `string`, así que `request.query.tag.length` y `request.query.tag.toUpperCase()` se leen como en cualquier otra cadena.

Un campo de esquema puede ser un enum. El valor que cruza la red es el valor que declara el miembro: `Role.Admin = "admin"` es la cadena JSON `"admin"`, y `Level.High = 9` es el número `9`. Nada convierte un número en el nombre de su miembro a la salida, porque entonces el valor en el programa y el valor en la red serían dos cosas distintas y solo una de ellas sería la que compara un `switch`. La lectura es donde esto se paga: un valor del tipo correcto que no es un miembro declarado se rechaza con `400`, igual que un valor de tipo equivocado. Sin esa comprobación la red podría entregar al programa un `Level` con `5`, que no es ningún miembro que el programa declaró, y todo `switch` sobre él se caería. Un enum cuyos miembros llevan payload no se acepta, porque no tiene un único valor que ser; el rechazo nombra el campo y dice por qué.

Un campo de esquema de respuesta también puede ser una unión discriminada: cada variante un objeto, con un campo compartido que lleva un literal distinto en cada una. Sale como el objeto que su variante viva ya es, así que `{ kind: "done"; value: i32 } | { kind: "failed"; reason: string }` escribe `{"kind": "done","value": 42}` y nada más. No hay envoltorio, porque el campo discriminante es lo que le dice al lector qué variante recibió. Un esquema de petición la lee de vuelta del mismo modo, eligiendo la variante por su discriminante. Un cuerpo cuyo discriminante no nombra ninguna variante declarada se rechaza con `400`, exactamente como un valor de tipo equivocado: el cable no puede entregarle al programa una variante que nunca declaró.

Solo `body` es JSON escrito por quien envía. Los path params, la query, las cabeceras y las cookies llevan texto: `?limit=25` son dos caracteres en la red, y `/users/123` no dice si `123` es un identificador o un recuento. El esquema declarado decide cómo se lee ese texto, así que `id: string` acepta `/users/123` y `limit: i32` lee `?limit=25` como número. Un valor que el esquema no puede leer se rechaza con `400`, nombrando el canal que falló, como `Invalid request query`. Un cuerpo JSON sigue siendo estricto: `{"limit": "25"}` no satisface `limit: i32`.

<!-- @include: ../../../.fragments/modules/http/request-channels.md -->

## Peticiones rechazadas

Una petición que no encaja con sus esquemas se responde con `400` y un cuerpo que lleva `statusCode`, `code`, `message` e `issues`. El mensaje nombra el canal que falló, y cada issue nombra el `field`, el `validator` que la rechazó y el motivo. Una petición rechazada antes de que se ejecutara cualquier decoration, como una cuyo cuerpo no es JSON, tiene `issues` vacío.

<!-- @include: ../../../.fragments/modules/http/validation-issues.md -->

## Handlers en otro módulo

Un controller importado puede dejar sin tipo los dos parámetros del handler. La ruta proporciona la forma de `req` a partir de sus esquemas de petición y el tipo de `res` a partir de los estados de respuesta declarados, así que el controller no necesita importar esos tipos de esquema ni repetir la unión de respuestas. El controller todavía puede importar los valores que usa, como `HttpStatus`, y se admiten las anotaciones explícitas de parámetros. Un controller inferido tiene un único contrato de ruta; use controllers separados o una firma compartida explícita cuando las rutas tengan esquemas diferentes.

<!-- @include: ../../../.fragments/modules/http/named-handler.md -->

## Dirección del cliente y proxies de confianza

El rate limit de una ruta se indexa por la dirección del cliente, y esa dirección es el peer de la conexión. `app.trustedProxies([...], identityHeader?)` lista los peers cuyas cabeceras de reenvío el servidor cree, como direcciones (`10.0.0.1`) o redes en formato CIDR (`10.0.0.0/8`).

<!-- @include: ../../../.fragments/modules/http/trusted-proxies.md -->

Detrás de un proxy listado el servidor lee las cabeceras de reenvío en su lugar: recorre cada cadena desde la derecha y salta los saltos que también son de confianza, así que la primera dirección que no lo es será la más cercana por la que respondió la cadena de confianza. `Forwarded` y `X-Forwarded-For` se leen ambas, y ambas tienen que nombrar al mismo cliente. Un proxy suele escribir una de ellas y no elimina la otra, así que dejar que cualquiera decida por su cuenta permitiría que un llamante enviara la cabecera sobrante y eligiera su propia dirección, derrotando el límite, el baneo y el tope compartido a la vez. Cuando las dos discrepan la petición cuenta contra el peer, la única dirección que el servidor observó por sí mismo. Pase `identityHeader` cuando el proxy use otra cabecera de dirección, como `"x-client-ip"`; reemplaza las cabeceras estándar, pasa a ser la única leída y acepta la misma cadena de direcciones separada por comas. Solo se lee después de que el peer coincide con la lista de confianza, y una dirección malformada vuelve al peer en lugar de convertirse en una identidad nueva. Desde cualquier otro peer esas cabeceras se ignoran, porque un cliente puede escribirlas, y creerlas dejaría a un llamante presentar una dirección nueva por petición y nunca alcanzar un límite. Una entrada malformada rechaza la lista entera en vez de descartar una red en silencio, y el servidor se detiene en vez de arrancar con la política equivocada.

## Ciclo de vida

`app.listen(port, options?)` devuelve `Result<HttpServer, HttpError>` después de enlazar el puerto. Gestione un error de inicio con `onfail`; su variable contiene el valor `HttpError`. En una función `void`, el bloque de error retorna de esa función, mientras que una llamada correcta proporciona un handle `HttpServer`.

<!-- @include: ../../../.fragments/modules/http/listen-example.md -->

`server.port()` informa del puerto realmente enlazado, incluso cuando el puerto solicitado era `0`. `server.close()` deja de aceptar conexiones nuevas y se resuelve cuando terminan las solicitudes que ya estaban en curso. El runtime mantiene activo el listener después de que `main` retorna y espera SIGINT o SIGTERM; la señal cierra los servidores activos y completa las solicitudes, con el límite de `shutdownTimeout`. Una segunda señal termina el proceso de inmediato.

`shutdownTimeout` limita cuánto espera `close`. Después del plazo, se cierran las conexiones que siguen abiertas. Sin `shutdownTimeout`, la espera no tiene límite. Un cierre que termina solicitudes en curso informa `ERR_HTTP_SHUTDOWN_TIMEOUT`; cerrar un handle no válido informa `ERR_HTTP_SHUTDOWN`.

<!-- @include: ../../../.fragments/modules/http/create-router/error-handling.md -->

## Configuración de ruta

Las opciones de ruta pueden incluir `config`, un objeto tipado de ajustes por ruta. Tanto `config` como cada una de sus claves son opcionales, pero una clave presente debe estar completa.

<!-- @include: ../../../.fragments/modules/http/route-config.md -->

`bodyLimit` es un tamaño en bytes escrito como cadena que sustituye el valor por defecto del servidor, de 2 MB. Las unidades decimales (`kb`, `mb`, `gb`) son potencias de 1000 y las binarias (`kib`, `mib`, `gib`) son potencias de 1024, y `"off"` desactiva el límite por completo, lo que acepta cualquier tamaño de cuerpo y es una concesión frente a la denegación de servicio que solo conviene en una ruta interna de confianza. Un cuerpo por encima del límite efectivo se rechaza con `413`.

`rateLimit` declara `max`, un recuento entero de peticiones por identidad de cliente, y `timeWindow`, una cadena de duración como `"1m"`, `"30s"`, `"500ms"` o la forma verbosa `"10 seconds"`; ambas son obligatorias cuando `rateLimit` está presente. `globalMax` es un tope entero opcional compartido por todas las identidades que usan ese límite. `ban` es un recuento opcional no negativo de peticiones consecutivas que superan el límite de la propia identidad; después de ese recuento, la identidad recibe `403` con `rate_limit_banned` hasta que `banWindow` expire. `ban: 0` bloquea en la primera petición excedida y `banWindow` usa `timeWindow` por defecto. Agotar `globalMax` nunca suma una ocurrencia de bloqueo, así que un servidor ocupado no puede bloquear a un cliente que se comporta correctamente. El límite es un token bucket por identidad de cliente, aplicado antes de leer el cuerpo. Empieza con `max` tokens y se recarga continuamente durante `timeWindow`, así que una ráfaga no puede usar dos ventanas completas en su límite. Cuando `globalMax` está presente, un segundo token bucket empieza con esa cantidad de tokens y se comparte entre todas las identidades; una petición solo continúa cuando ambos buckets tienen crédito. Un cubo agotado responde `429` con `Retry-After`. La tabla de cubos de identidad está acotada, y una tabla llena descarta las identidades usadas menos recientemente en vez de rechazar a un cliente que nunca ha visto, así que llenarla desde muchas direcciones no convierte al limitador en la denegación de servicio que existe para evitar. Una identidad que cumple un baneo se conserva cualquiera que sea su antigüedad: una petición baneada se rechaza antes de tocar el reloj, así que es el registro más quieto de la tabla y sería lo primero en salir, terminando el baneo justo cuando llega la presión que lo provocó.

El `rateLimit` de una ruta sustituye a `app.rateLimit(...)` para esa ruta, en vez de sumarse a él, así que una ruta puede ser tan más laxa como más estricta que el valor por defecto del servidor. Definir el límite en el router es su lugar habitual: un límite que hay que repetir en cada ruta es un límite que alguna ruta acabará sin tener.

Los formatos de `bodyLimit` y `timeWindow` se validan en tiempo de compilación, así que se escriben inline.

## CORS

`app.cors(options)` instala el middleware de CORS sobre todo el router y atiende las peticiones de preflight `OPTIONS` antes del despacho de ruta. `origin`, `allowedHeaders` y `exposedHeaders` son arrays de cadenas; `methods` está restringido a los literales de método HTTP en mayúsculas, así que una errata es un error de tipos. `allowedHeaders` sigue siendo un array de cadenas corriente a propósito, porque una aplicación puede autorizar una cabecera que ella misma inventó. `maxAge` va en segundos, y `0` la deja sin valor.

## CSRF

`app.csrf({ cookie, header })` habilita una comprobación double-submit opcional para rutas autenticadas por cookie. Antes de que una petición insegura llegue a un handler, el runtime compara la cookie y la cabecera configuradas en tiempo constante; un valor ausente o diferente devuelve `403` con `csrf_invalid`. La generación del token y su vínculo con la sesión siguen siendo política de la aplicación. Consulta [CsrfOptions](./csrf-options.md).

`"*"` se acepta en `origin` y `allowedHeaders`, pero no junto a `credentials: true`. El navegador rechaza ese par por completo en vez de estrecharlo, así que toda petición con credenciales fallaría; el compilador señala el `"*"` que lo causó.

::: danger error
<!-- @include: ../../../.fragments/modules/http/cors-credentials.md -->
:::

Los orígenes se comprueban mientras siguen siendo literales. El navegador envía `Origin: <esquema>://<host>[:<puerto>]` y nada más, así que una entrada escrita con ruta, query, barra final o sin esquema se compara con un valor que nunca puede contenerlas, y en silencio no coincide con nada. Ese fallo es invisible en tiempo de ejecución, y por eso aquí es un error de compilación. Una entrada que lleva una ruta recibe un Quick Fix que la recorta; la falta de esquema no, porque `http` y `https` son orígenes distintos y adivinar produciría una regla que parece correcta y no coincide con nada.

Una lista `origin` vacía también se rechaza: no coincide con ningún origen, lo que rechaza toda petición cross-origin, y un router que quisiera eso sencillamente no llamaría a `cors`.

Las mismas reglas se aplican de nuevo al arrancar, para valores que no son literales. Los métodos de registro de rutas `get`, `post`, `put`, `patch`, `delete`, `head`, `options`, `connect`, `query`, `trace`, `fallback`, `ws` y `docs`, además de `onResponse`, `cors`, `csrf`, `fetchPolicy`, `rateLimit` y `trustedProxies`, devuelven `Result<void, HttpError>` cuando el runtime rechaza una ruta o configuración. La inicialización puede propagar estos fallos con `?` o manejarlos con `match` antes de empezar a escuchar.

## Peticiones salientes

`app.fetchPolicy(options)` indica adónde puede ir `fetch`. Nada se rechaza hasta que el programa lo escribe: hablar con `10.0.0.5` o con `localhost` es habitual, y un runtime que lo rechazara por defecto enseñaría a su autor a desactivar la protección por completo.

<!-- @include: ../../../.fragments/modules/http/fetch-policy.md -->

`denyPrivate` rechaza destinos accesibles solo desde dentro: loopback, los rangos privados, link-local y el resto del espacio en el que ningún host de internet pública responde. `allow` es una lista de excepciones, no una lista de permitidos. Nombra hosts que pueden alcanzarse aunque su dirección fuera rechazada, así que añadir una entrada solo amplía lo que el programa alcanza, nunca lo restringe.

La política existe para una forma concreta de programa: una ruta que entrega a `fetch` una URL elegida por el cliente. La petición sale desde dentro de la red y por eso alcanza aquello que la red protegía. `169.254.169.254` es el servicio de metadatos de la nube, que responde credenciales a quien logre alcanzarlo y trata la accesibilidad como autenticación; una caché o un panel de administración en una dirección privada está protegido igual.

Un destino se juzga por la dirección a la que resuelve, no por cómo se escribe. Comprobar el nombre del host no sería comprobación alguna: nada impide que un nombre público resuelva a una dirección privada, y una política que confiara en la grafía aprobaría el servicio de metadatos en cuanto alguien apuntara un dominio hacia él. Por el mismo motivo, cada salto de una redirección se comprueba como el primero, porque una redirección nombra un destino que el programa nunca escribió.

Un destino rechazado rechaza la promise con `ERR_HTTP_FETCH_BLOCKED`, que es un código propio y no el mismo de un nombre que no resuelve: a quien se le dice que su nombre de host está mal buscaría en el lugar equivocado algo que decidió su propia política.

## Cabeceras de seguridad

Las cabeceras de seguridad se escriben como cualquier otra cabecera de respuesta, con `reply.header(name, value)`. No hay un conjunto predefinido que las añada: lo que el navegador recibe es lo que el handler escribió, y el editor ofrece las cuatro por nombre, con sus valores, dentro de `reply.header("`.

<!-- @include: ../../../.fragments/modules/http/security-headers.md -->

<!-- @include: ../../../.fragments/modules/http/security-headers-table.md -->

Cada una es una instrucción estrecha para el navegador, y un cliente que la ignora no se ve afectado.

El editor también ofrece `content-security-policy` y `strict-transport-security`, y deja el valor de ambas a la aplicación. Una política lo bastante genérica para toda aplicación o es tan laxa que no protege nada o es tan estricta que rompe la página, así que lo que debe decir es decisión de la aplicación. `strict-transport-security` es una promesa sobre TLS: envíala solo desde detrás de TLS, porque sobre HTTP plano pide al navegador que rechace la conexión que funciona.

Nada de esto hace que una aplicación sea segura frente a cross-site scripting. Estas cabeceras reducen lo que el navegador hace con una respuesta; no dicen nada sobre lo que la aplicación pone en ella.

Una respuesta que el propio router produjo, como un `404` para una ruta sin coincidencia, no pasó por ningún handler y por eso no lleva ninguna de ellas.

## Respuestas que no encajan con su schema

Cuando un handler produce una respuesta que no satisface el schema declarado por la ruta, el cliente recibe un `500` genérico con `code: "internal_server_error"` y nada más. El cliente no causó el fallo y no puede actuar sobre él, y los nombres de campo implicados describen el schema del propio servidor, no algo que el cliente enviara.

El campo y la regla que fallaron van a stderr, donde los verá quien opera y puede corregirlo. Esto es deliberadamente distinto del contrato del `400`: una petición rechazada sí nombra el campo y la regla en su cuerpo, porque allí el fallo es del cliente y es él quien lo corrige.

## Pre-handlers

Las opciones de ruta pueden incluir `preHandler`, un array literal de funciones síncronas nombradas. Importe `HttpRequest` y `HttpResponse` al declarar un pre-handler. Recibe el `HttpRequest` sin procesar y devuelve `HttpResponse | true`: `true` continúa al siguiente pre-handler o controller, mientras que un `HttpResponse` detiene la cadena y se envía inmediatamente. La cadena queda fija en tiempo de compilación y está disponible tanto en las rutas como en `fallback`.

Para la generación OpenAPI y la configuración de Swagger UI, incluida la autenticación, los estilos personalizados y la protección de la documentación, consulte [OpenAPI / Swagger](./openapi-swagger.md).

## Manejo de errores

Declara el cuerpo de error en la aplicación y mapea un error de dominio en el límite del controller al status HTTP elegido. El status HTTP ya está disponible al cliente como status de la respuesta y no necesita duplicarse en JSON.

Un handler no puede dejar escapar un error: el servidor lo llama sin ningún `catch` alrededor, así que un handler que lanza, inline o referenciado por nombre, se rechaza con `E3298`. Captura el error dentro del handler y responde con el status que representa.

<!-- @include: ../../../.fragments/modules/http/create-router/error-handling.md -->
