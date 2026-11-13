---
title: HttpRouter.onResponse
description: Observe respuestas terminadas de tyrus:http desde la configuración del servidor.
---

# HttpRouter.onResponse

`app.onResponse(hook)` registra un callback síncrono con nombre para cada respuesta servida por ese router. Regístrelo en la configuración del servidor antes de `app.listen(...)`, y no dentro de una ruta.

## Firma

<!-- @include: ../../../.fragments/modules/http/on-response/signature.md -->

El callback se ejecuta una vez cuando el servidor termina de producir o descarta el cuerpo de la respuesta. Esto incluye una respuesta `StreamBody`. Observa el estado final y la duración, pero no puede cambiar la respuesta y no es un `preHandler` de ruta.

<!-- @include: ../../../.fragments/modules/http/on-response/signature-2.md -->

Se permite registrar más de un hook. Los hooks se ejecutan en el orden de registro. El callback puede realizar cualquier trabajo síncrono permitido por la aplicación, como actualizar una métrica, escribir un registro de auditoría o reenviar datos a un adaptador de tracing.

::: danger error
<!-- @include: ../../../.fragments/modules/http/on-response/signature-3.md -->
:::

El hook debe ser una función con nombre. Un callback con nombre proporciona al servidor un punto de entrada nativo estable e impide que una closure sobreviva al estado de ruta que capturó.
