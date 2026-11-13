---
title: VS Code
description: Experiencia de edición de tyrus en VS Code con diagnósticos, completion, hover, inlay hints, semantic tokens, Quick Fixes y code lenses de Quick Fix.
---

# VS Code

La extensión de VS Code es un cliente para el servidor compartido `tyrus lsp`. Debe sentirse como una experiencia nativa de lenguaje manteniendo las reglas semánticas en el compilador y el Language Server.

## Instalar y abrir

Haz build o instala la extensión desde `editors/vscode`, luego abre un proyecto que contiene archivos `.tyr`.

Instala la [extensión Tyrus publicada en VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=tyrus-lang.tyrus).

<!-- @include: ../../.fragments/tooling/install.md -->

La extensión se selecciona automáticamente como formatter predeterminado para archivos `.tyr` y habilita el formato al guardar. Estos valores predeterminados no agregan entradas a `settings.json`, y la configuración explícita del usuario o workspace tiene prioridad. Abre Settings con `Ctrl+,` y busca `tyrus`, o busca `@ext:tyrus.tyr-language-support` para mostrar solo la configuración de la extensión.

El logotipo de tyrus identifica la extensión en la vista Extensions, y los archivos `.tyr` exponen el icono del lenguaje tyrus en el Explorer. Un tema de iconos de archivo de terceros puede reemplazar intencionalmente los iconos aportados por los lenguajes con su propio diseño.

::: tip Inteligencia compartida
VS Code, Neovim, Zed e IntelliJ/WebStorm deben recibir los mismos diagnósticos y code intelligence desde `tyrus lsp`.
:::

## Diagnósticos

Los diagnósticos del compilador y del linter aparecen como squiggles en el editor y en el panel Problems. De forma predeterminada, la extensión también agrega un fondo translúcido a toda la línea: rojo para errores y amarillo para warnings. El squiggle y su range preciso de origen permanecen visibles.

Define `tyrus.diagnostics.highlightWholeLine` como `false` en `settings.json` para desactivar el fondo de línea completa sin cambiar los diagnósticos. Como alternativa, presiona `Ctrl+,`, busca `tyrus`, encuentra **Tyrus › Diagnostics: Highlight Whole Line** y desmarca la casilla.

::: danger error
<!-- @include: ../../.fragments/tooling/diagnostics.md -->
:::

Feedback esperado en el editor:

| Origen | Severidad | Ejemplo |
|--------|-----------|---------|
| Compilador | Error | `expected i32, found string` |
| Linter | Warning | `retries is never reassigned` |
| Schema de formatter/lint config | Error o warning | Claves desconocidas o inválidas en `tyrus-fmt.json` y `tyrus-lint.json`. |

## Completion

Completion incluye símbolos locales, símbolos importados, exports de módulos nativos, keywords, tipos primitivos, validator decorators, campos de objeto, claves de opción de ruta como `handler`, `response`, `params` y `config`, claves anidadas de rate limit como `max`, `timeWindow` y `globalMax`, y miembros de arrays, strings, `Regex`, `HttpRouter` y `HttpStatus`. El hover sobre una opción de ruta explica su valor esperado y su finalidad.

<!-- @include: ../../.fragments/tooling/completion.md -->

## Hover y Signature Help

Hover muestra tipos inferidos, firmas de imports nativos, campos de interface, documentación de validators y diagnósticos. Signature help muestra nombres de parámetros y el argumento activo mientras completas una llamada.

<!-- @include: ../../.fragments/tooling/hover.md -->

::: details Ejemplo de hover
<!-- @include: ../../.fragments/tooling/hover-example.md -->
:::

## Inlay hints

El Language Server proporciona inlay hints de nombres de parámetros y tipos inferidos.

<!-- @include: ../../.fragments/tooling/inlay-hints-source.md -->

Hints esperados:

<!-- @include: ../../.fragments/tooling/inlay-hints-expected.md -->

## Quick Fixes

Quick Fixes están disponibles cuando la edición es determinística.

<!-- @include: ../../.fragments/tooling/quick-fix.md -->

Ejemplos incluyen importar un símbolo ausente, eliminar imports no usados, aplicar `prefer-const` y tomar prestado un valor que se movería.

## Code Lenses de Quick Fix

Cuando un diagnóstico en `.tyr` ofrece al menos un Quick Fix determinístico, el Language Server también lo expone como un code lens encima de la línea con el problema. El lens muestra exactamente el título del Quick Fix (por ejemplo, `Change let to const`), y hacer clic en él aplica la edición inmediatamente sin abrir el menú de Quick Fix. Cuando un diagnóstico tiene más de una corrección, cada corrección obtiene su propio lens al lado de los demás. Los Quick Fixes de archivos `.env` siguen disponibles solamente mediante la bombilla.

El Quick Fix normal sigue funcionando mediante la bombilla y los comandos estándar del editor; los lenses solo agregan un punto de entrada más cerca del código.

Define `tyrus.codeLens.quickFixes.enable` como `false` en `settings.json` para ocultar los code lenses de Quick Fix sin cambiar los propios Quick Fixes. El cambio en esta configuración surte efecto después de recargar la ventana.

## Semantic highlighting

Semantic tokens mejoran la coloración de símbolos, tipos primitivos, tipos nativos, interfaces, type aliases, keywords, literal keywords y palabras reservadas para recursos futuros del lenguaje.

::: warning Highlighting no es validación semántica
Los colores de sintaxis ayudan a la lectura, pero los diagnósticos del compilador y del linter son la fuente de verdad para la corrección.
:::
