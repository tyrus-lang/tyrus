---
title: Registro automático de rutas
description: Descubra y registre plugins de ruta en `src/routes/`.
---

# Registro automático de rutas

En un proyecto de API, el entrypoint de Tyrus descubre recursivamente los módulos de ruta dentro de `src/routes/`. Cada módulo puede exportar una o más funciones con la firma `(router: HttpRouter) => void`; Tyrus llama a cada función con el router de la aplicación cuando se ejecuta `createRouter()`, antes de que el servidor empiece a escuchar. El export default sigue siendo compatible. No hace falta crear `index.tyr` ni importar cada ruta.

<!-- @include: ../../../.fragments/modules/http/automatic-routes.md -->

## Discovery rules

Un archivo `.tyr` en `src/routes/` sin una función exportada que coincida con la firma de ruta se trata como módulo auxiliar y no se invoca. Un export default en esa carpeta debe coincidir con la firma del plugin de ruta o el proyecto falla al cargarse. Los archivos terminados en `.test.tyr` se excluyen de la detección.

Los exports fuera de `src/routes/` nunca se registran automáticamente. Importe e invoque esos módulos explícitamente cuando haga falta.
