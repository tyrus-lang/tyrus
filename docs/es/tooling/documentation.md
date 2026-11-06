---
title: Mantenimiento de la Documentación
description: Checklist de release y política de mantenimiento para el sitio de documentación de tyrus.
---

# Mantenimiento de la Documentación

La documentación de tyrus se versiona junto con el compilador y el runtime. La documentación actual vive en `docs/`, los releases archivados se generan a partir de Git tags, y el sitio publicado debe exponer la documentación estable más reciente en `/`.

## Ownership

Los mantenedores core son responsables de la metadata de versión de la documentación, las actualizaciones del checklist de release y la revisión de páginas que describen lenguaje, compilador, runtime, CLI, formatter, linter, LSP o comportamiento de módulo nativo.

El ownership de traducción sigue la misma regla hasta que existan mantenedores dedicados de traducción. Inglés permanece como lenguaje fuente para comportamiento técnico, y las páginas localizadas deben mantener contenido localizado integral cuando la ruta exista.

## Checklist de Release

- Actualiza la documentación para cada cambio visible al usuario antes de crear la release tag.
- Mantén la documentación actual en `docs/` y no crees carpetas permanentes `docs/v0.1.0/` en el branch principal.
- Confirma que `docs/.vitepress/versions.ts` tiene la versión actual como `latestDocsVersion` y versiones archivadas en `previousDocsVersions`.
- Ejecuta `npm run docs:build` antes de cada release tag.
- Ejecuta `npm run docs:build:versions` antes de publicar el sitio de documentación.
- Verifica `/`, `/pt/`, `/es/`, links de versión, links de sidebar, assets de logo, assets de favicon y búsqueda local en el sitio buildado.
- Publica solo `docs/.vitepress/dist` o el directorio de salida estática específico del provider.

## Reglas de Revisión

- No documentes comportamiento incompleto o experimental como estable.
- Marca recursos experimentales explícitamente y describe el límite actualmente implementado.
- Mantén páginas de API cerca del árbol público de módulos, como `docs/modules/fs/read-file.md`, para que links de sidebar, resultados de búsqueda y ownership de fuente permanezcan previsibles.
- No generes páginas de referencia de API automáticamente hasta que el compilador o LSP exponga metadata estable para firmas, diagnósticos, exports de módulo y ejemplos.
- Prefiere páginas pequeñas con deep links estables en vez de una gran página de API.
- Mantén snippets ejecutables o directamente alineados con un ejemplo ejecutable cuando el recurso tenga comportamiento de runtime.

## Referencia Generada en el Futuro

La documentación de referencia generada queda intencionalmente diferida hasta que tyrus exponga metadata estable del compilador o del LSP. El generador futuro debe leer exports estructurados de módulos, firmas, códigos de diagnóstico y ejemplos en vez de hacer scraping de Markdown o comentarios en el código.
