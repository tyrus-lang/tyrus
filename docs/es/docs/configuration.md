---
title: Configuración del Proyecto
description: Estructura del proyecto, tyrus.json, scripts de documentación y flujo de release.
---

# Configuración del Proyecto

Un proyecto tyrus generado tiene un manifiesto pequeño, un punto de entrada, archivos de entorno tipados opcionales y un directorio `target/` para artefactos generados.

## Estructura del Proyecto

<!-- @include: ../../.fragments/docs/configuration-project-structure.md -->

Este es el proyecto `script` predeterminado. La plantilla `cli` agrega `src/prompts/setup.tyr` y usa `__tests__/example.test.tyr` en lugar de la prueba predeterminada. La plantilla `api` agrega `src/controllers/user.tyr`, `src/models/user.tyr`, `src/repositories/user.tyr`, `src/routes/user.tyr`, `src/schemas/user.tyr` y `src/services/user.tyr`, y también usa `__tests__/example.test.tyr`. La plantilla `api` crea el directorio `data/` cuando se selecciona SQLite.

`src/main.tyr` es el archivo de entrada obligatorio y debe declarar `const main = (): void => { ... };`. La ruta y la firma de la función son fijas. Los artefactos de compilación se escriben en `target/debug/` de forma predeterminada y en `target/release/` con `tyrus build --release`. `tyrus dev` usa `target/debug/`.

## tyrus.json

<!-- @include: ../../.fragments/docs/configuration-tyrus-json.md -->

`project` describe metadata del paquete. `build.optimize` selecciona el modo de optimización predeterminado. `development.https` hace que un servidor arrancado sin `tls` sirva HTTPS en `localhost` mientras el programa se ejecuta con `TYRUS_ENV=development`, y no hace nada en cualquier otro entorno; consulta [createRouter](../modules/http/create-router.md). `runtime.workerThreads` limita cuántos hilos de trabajo puede usar el runtime asíncrono y, si se omite, adopta el número de CPUs lógicas. `security.envFile` habilita análisis tipado de entorno. `security.envSealed` cambia la carga de entorno en runtime por un artefacto sealed cifrado.

`scripts` asigna nombres de scripts del proyecto a comandos de shell. Los manifiestos generados incluyen las entradas `fmt`, `lint` y `test`. Añade un script personalizado como valor string y ejecútalo por su nombre con `tyrus run <name>`. Por ejemplo, esta entrada hace que `tyrus run example` ejecute `tyrus run example.tyr` desde el directorio del proyecto:

```json
{
  "scripts": {
    "example": "tyrus run example.tyr"
  }
}
```

Los argumentos para el comando configurado van después de `--`, como en `tyrus run example -- --verbose`. `tyrus --init` conserva los scripts personalizados al agregar los valores predeterminados que faltan; `tyrus --default` restablece el manifiesto con los valores predeterminados generados para el proyecto y elimina los scripts personalizados.

## Sitio de Documentación

La documentación del repositorio vive en `docs/` y se buildea con VitePress:

<!-- @include: ../../.fragments/docs/configuration-docs-commands.md -->

La documentación actual se publica en `/`. Releases archivados se generan desde Git tags en rutas como `/v0.1.0/`. La metadata de versión vive en `docs/.vitepress/versions.ts`.
