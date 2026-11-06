---
title: OpenAPI / Swagger
description: Genere y configure la documentación de la API con OpenAPI y Swagger UI.
---

# OpenAPI / Swagger

`app.docs()` pertenece a `tyrus:http`. Genera un documento OpenAPI 3.2 a partir de las rutas registradas en el router y sirve una página Swagger UI desde la propia aplicación. No hace falta un servicio externo de documentación.

## Habilitar la documentación

Llame a `app.docs(options)` en el router. La página predeterminada es `/docs`, y el documento OpenAPI está disponible en `/docs/openapi.json`. Defina `path` para usar otra ruta base. Se incluyen las rutas registradas antes o después de esta llamada.

`app.docs(options)` devuelve `Result<void, HttpError>` porque el runtime puede rechazar una configuración de documentación no válida. Propaga el resultado con `?` durante la configuración del router o manéjalo con `match`.

| Opción | Tipo | Predeterminado | Finalidad |
| --- | --- | --- | --- |
| `path` | `string` | `"/docs"` | Ruta base de Swagger UI, del documento OpenAPI y de los recursos incluidos. Debe comenzar con `/`. |
| `title` | `string` | `"API"` | Título de la API que muestra Swagger UI y que se escribe en el objeto OpenAPI `info`. |
| `version` | `string` | `"1.0.0"` | Versión de la API que se escribe en el objeto OpenAPI `info`. |
| `description` | `string` | vacío | Descripción de la API que se escribe en el objeto OpenAPI `info`. |
| `auth` | objeto literal | ninguno | Esquemas de seguridad que muestra Swagger UI y que declara el documento OpenAPI. No autentica las solicitudes por sí solo. |
| `css` | `string` | vacío | Código CSS insertado en la página después de la hoja de estilos incluida de Swagger UI. Pase el texto CSS, no la ruta de un archivo. |
| `preHandler` | array de funciones con nombre | ninguno | Guards síncronos ejecutados antes de servir la página, el documento OpenAPI y los recursos de Swagger UI. |

El compilador valida el objeto inline. No existe una interface con nombre que se pueda importar. Estas son todas las opciones que `app.docs` acepta actualmente.

## Contenido del documento

El documento describe el método y la ruta de cada endpoint, los parámetros de ruta, los schemas declarados de `params`, `query`, `headers` y `cookies`, los cuerpos de solicitud, los status de respuesta y los schemas de respuesta. Las interfaces y enums se convierten en schemas con nombre reutilizables. Los cuerpos JSON de solicitud incluyen ejemplos generados que cumplen las restricciones declaradas en su schema. `Instant` se describe como una cadena de fecha y hora.

Añada `tags` a las opciones de ruta para agrupar operaciones en secciones con nombre en Swagger UI. Las rutas sin tags aparecen en `default`.

```tyrus
router.get("/users", {
    tags: ["users"],
    response: { 200: User[] },
    handler: listUsers,
});
```

El método HTTP `QUERY` se representa como una operación OpenAPI 3.2, incluido el cuerpo de solicitud declarado. Las rutas WebSocket aparecen como operaciones `GET` con respuesta `101 Switching Protocols` y la extensión `x-tyrus-websocket`. El documento se construye a partir de las declaraciones de ruta, sin exigir una definición OpenAPI separada para cada una.

## Configurar la autorización de Swagger UI

Use `auth` para describir cómo se autoriza un cliente en Swagger UI. Los esquemas aparecen en el control **Authorize** y se escriben en las definiciones de seguridad de OpenAPI. La descripción del esquema no verifica el acceso: proteja las rutas de la API con la lógica de autenticación y autorización de la aplicación.

### Token bearer

```tyrus
auth: { bearer: true, bearerFormat: "JWT" }
```

`bearerFormat` es opcional y solo se puede usar con `bearer: true`.

### Clave de API

```tyrus
auth: { apiKey: { name: "x-api-key", in: "header" } }
```

`name` es el nombre del header, parámetro de query o cookie de la clave. `in` debe ser `"header"`, `"query"` o `"cookie"`; prefiera un header para las credenciales.

## Personalizar la página con CSS

`css` recibe código CSS, no la ruta de una hoja de estilos. El runtime lo coloca en un elemento `<style>` después de la hoja de estilos de Swagger UI, así que estas declaraciones pueden sobrescribir los estilos predeterminados.

```tyrus
css: ".swagger-ui .topbar { background: #16324f; } .swagger-ui .info { margin: 2rem 0; }"
```

## Proteger los endpoints de documentación

`preHandler` recibe el `HttpRequest` sin procesar y debe ser un array literal de funciones síncronas con nombre y la firma `(request: HttpRequest) => HttpResponse | true`. Devuelva `true` para servir el recurso de documentación solicitado. Devuelva un `HttpResponse` para detener la solicitud, por ejemplo con `HttpStatus.Forbidden`. El mismo guard protege la página HTML, `/openapi.json`, la hoja de estilos y el bundle de scripts.

Use dentro del guard el validador de sesión o credenciales real de la aplicación. Este ejemplo presupone que `isAuthorizedAdmin` es una función que ya existe en la aplicación; `tyrus:http` no la proporciona.

<!-- @include: ../../../.fragments/modules/http/openapi-swagger/pre-handler-example.md -->

La opción `auth` anterior solo indica a Swagger UI cómo enviar credenciales al probar una operación. No protege la página de documentación, no valida credenciales y no sustituye la autorización de las rutas. Use `preHandler` para controlar el acceso a la documentación y las comprobaciones normales de autorización de la aplicación para proteger las operaciones de la API.

## Ejemplo completo de configuración

Este ejemplo combina los metadatos de la API, un esquema bearer, CSS personalizado y el guard de documentación. Sustituya el título y el esquema de autorización de ejemplo por los que use la aplicación.

<!-- @include: ../../../.fragments/modules/http/openapi-swagger/setup-example.md -->
