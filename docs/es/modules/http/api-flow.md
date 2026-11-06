---
title: API Flow
description: Sigue una solicitud de User por el main, Model, schemas, routes, controllers, services y repository de una API SQLite.
---

# API Flow

Esta guía sigue solicitudes de User por una API SQLite, desde `tyrus.json` y el inicio del servidor hasta la operación SQL o del Model y la respuesta HTTP. El código usa la misma estructura y formato que el proyecto `api-sqlite`.

## Estructura del proyecto

El directorio `src/routes/` se descubre automáticamente cuando se ejecuta `createRouter()`. Tyrus llama a cada función de ruta exportada con el router de la aplicación, así que `main.tyr` no necesita una importación por cada módulo de ruta.

## Ejemplo completo

El ejemplo expone endpoints de listado, paginación numérica, búsqueda, lectura, creación, actualización, reemplazo y eliminación. El Model proporciona el `id` y los timestamps generados; el repository usa `db.query` para SQL y métodos del Model para operaciones tipadas con registros.

<!-- @include: ../../../.fragments/modules/http/api-flow/complete-example.md -->

## Flujo de la solicitud

- `main.tyr` crea el router, configura los metadatos de Swagger e inicia el servidor con `app.listen`.
- `models/users.tyr` mapea la interfaz `User` a la tabla `users`; el `id`, `createdAt` y `updatedAt` generados no necesitan declararse en la interfaz.
- `schemas/user.tyr` define las entradas de las rutas, los cuerpos de respuesta y los cuerpos de error; los errores de la aplicación declaran `code` y `message`.
- `routes/user.tyr` declara los métodos HTTP, los schemas de validación y respuesta y el controller de cada endpoint.
- `controllers/user.tyr` maneja los códigos HTTP y convierte el éxito o el error de `Result` en cuerpos de respuesta.
- `services/user.tyr` nombra las operaciones de la aplicación y devuelve resultados asíncronos tipados.
- `repositories/user.tyr` controla el acceso a la base de datos y devuelve valores `Result`; usa parámetros enlazados para SQL y operaciones de Model para los registros.

Los nombres y rutas de archivo del ejemplo son ilustrativos; mantén los módulos de ruta en `src/routes/` para el registro automático. Consulta el [registro automático de rutas](./automatic-routes.md), [`tyrus:db`](../db/) y [OpenAPI / Swagger](./openapi-swagger.md) para ver los contratos relacionados.
