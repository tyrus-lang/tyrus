---
title: Formateador
description: Referencia completa de comandos y configuración de tyrus-fmt.
---

# Formateador

`tyrus fmt` formatea archivos fuente `.tyr` desde la raíz del proyecto. El formatter lee `tyrus-fmt.json` cuando existe; cada clave es opcional y las claves omitidas usan el default interno.

## Comandos

<!-- @include: ../../.fragments/tooling/formatter-commands.md -->

`tyrus fmt` reescribe archivos del proyecto en el lugar. `--check` verifica formato sin escribir archivos y es el modo adecuado para CI. `--stdout` imprime la salida formateada para rutas explícitas. `--init` crea o actualiza `tyrus-fmt.json` agregando opciones ausentes y preservando valores existentes. `--default` redefine `tyrus-fmt.json` a la configuración default completa.

## Configuración de ejemplo

<!-- @include: ../../.fragments/tooling/formatter-example-config.md -->

## Opciones

| Opción | Default | Valores | Finalidad |
|--------|---------|---------|-----------|
| [`array_expand`](#array_expand) | `"simple"` | `"simple"`, `"expand-all"`, `"off"` | Controla cuándo los arrays literales se expanden a varias líneas. |
| [`blank_lines`](#blank_lines) | `"collapse"` | `"collapse"`, `"off"` | Controla líneas en blanco entre imports, declaraciones, statements y bloques de flujo. |
| [`blank_lines_around_blocks`](#blank_lines_around_blocks) | `true` | `true`, `false` | Agrega líneas en blanco antes y después de statements de bloque y funciones. |
| [`blank_lines_around_decorations`](#blank_lines_around_decorations) | `true` | `true`, `false` | Agrega separación alrededor de campos decorados en interface/type. |
| [`brace_spacing`](#brace_spacing) | `true` | `true`, `false` | Imprime objetos como `{ x: 1 }` o `{x: 1}`. |
| [`brace_style`](#brace_style) | `"same-line"` | `"same-line"`, `"next-line"`, `"off"` | Controla llaves de apertura en funciones y bloques de control. |
| [`final_newline`](#final_newline) | `true` | `true`, `false` | Garantiza newline final en el archivo. |
| [`indent_style`](#indent_style) | `"spaces"` | `"spaces"`, `"tab"` | Selecciona espacios o tabs para indentación. |
| [`indent_width`](#indent_width) | `4` | entero `>= 0` | Número de espacios por nivel cuando `indent_style` es `"spaces"`. |
| [`imports_sort`](#imports_sort) | `true` | `true`, `false` | Ordena imports por ruta de módulo y mantiene imports `tyrus:*` primero. |
| [`line_width`](#line_width) | `90` | entero `>= 0` | Expande arrays, objetos, argumentos de llamada y ternarios que exceden el ancho configurado. |
| [`object_expand`](#object_expand) | `"simple"` | `"simple"`, `"expand-all"`, `"off"` | Controla cuándo los objetos literales se expanden a varias líneas. |
| [`pipeline_chain_layout`](#pipeline_chain_layout) | `"line-by-line"` | `"line-by-line"`, `"inline"`, `"off"` | Controla el layout de chains con más de un `|>`. |
| [`prefer_arrow_function`](#prefer_arrow_function) | `false` | `true`, `false` | Fuerza toda declaración de función de nivel superior a la forma arrow (`true`), o mantiene cada una en la forma en que fue escrita, mezclando `function` y arrow libremente (`false`). |
| [`quote_style`](#quote_style) | `"double"` | `"double"`, `"single"`, `"off"` | Selecciona comillas preferidas para strings o preserva las comillas y escapes originales de cada literal. |
| [`remove_unused_imports`](#remove_unused_imports) | `true` | `true`, `false` | Elimina specifiers de import no usados o declaraciones completas de import no usado. |
| [`trailing_comma`](#trailing_comma) | `true` | `true`, `false` | Controla comas finales en arrays y objetos multi-line. |

## `array_expand`

Controla cuándo los arrays literales se expanden a varias líneas.

<!-- @include: ../../.fragments/tooling/formatter-array-expand.md -->

## `blank_lines`

Controla líneas en blanco entre imports, declaraciones, statements y bloques de control.

<!-- @include: ../../.fragments/tooling/formatter-blank-lines.md -->

## `blank_lines_around_blocks`

Agrega líneas en blanco antes y después de statements de bloque y declaraciones de función.

<!-- @include: ../../.fragments/tooling/formatter-blank-lines-around-blocks.md -->

## `blank_lines_around_decorations`

`blank_lines_around_decorations` mantiene interfaces con muchos validators fáciles de leer.

<!-- @include: ../../.fragments/tooling/formatter-decorated-fields.md -->

## `brace_spacing`

Controla si se imprime un espacio entre llaves y su contenido.

<!-- @include: ../../.fragments/tooling/formatter-brace-spacing.md -->

## `brace_style`

Controla la posición de la llave de apertura en cuerpos de función y bloques de control.

<!-- @include: ../../.fragments/tooling/formatter-brace-style.md -->

## `final_newline`

Controla si se agrega un newline final al final del archivo.

<!-- @include: ../../.fragments/tooling/formatter-final-newline.md -->

## `indent_style`

Controla el estilo de indentación (espacios o tabs).

<!-- @include: ../../.fragments/tooling/formatter-indent-style.md -->

## `indent_width`

Controla el número de espacios por nivel de indentación.

<!-- @include: ../../.fragments/tooling/formatter-indent-width.md -->

## `imports_sort`

Controla si las declaraciones de import se ordenan alfabéticamente y se agrupan por tipo de origen.

<!-- @include: ../../.fragments/tooling/formatter-imports-sort.md -->

## `line_width`

Ancho máximo de línea. Arrays, objetos, argumentos de llamada y ternarios que exceden este límite se expanden a varias líneas.

<!-- @include: ../../.fragments/tooling/formatter-line-width.md -->

## `object_expand`

Controla cuándo los objetos literales se expanden a varias líneas.

<!-- @include: ../../.fragments/tooling/formatter-object-expand.md -->

## `prefer_arrow_function`

Fuerza toda declaración de función de nivel superior a la forma arrow o preserva la forma original.

<!-- @include: ../../.fragments/tooling/formatter-prefer-arrow.md -->

## `quote_style`

Selecciona comillas preferidas para strings. Con `"off"`, el formateador preserva las comillas y escapes originales de cada literal de cadena.

<!-- @include: ../../.fragments/tooling/formatter-quote-style.md -->

## `remove_unused_imports`

Elimina specifiers de import no usados o declaraciones completas de import no usado.

<!-- @include: ../../.fragments/tooling/formatter-remove-unused.md -->

## `trailing_comma`

Controla comas finales en arrays y objetos multi-line.

<!-- @include: ../../.fragments/tooling/formatter-trailing-comma.md -->

Con `true`, se agregan comas finales en arrays y objetos multi-line. Con `false`, no se agrega ninguna coma final.

## Antes y después

<!-- @include: ../../.fragments/tooling/formatter-before.md -->

<!-- @include: ../../.fragments/tooling/formatter-after.md -->

::: tip Usa `--check` en CI
`tyrus fmt --check` verifica formato sin escribir archivos, así que es el comando correcto para pull requests y pipelines de release.
:::

## `pipeline_chain_layout`

`pipeline_chain_layout` es útil para transformaciones funcionales porque cada paso sigue siendo fácil de revisar.

<!-- @include: ../../.fragments/tooling/formatter-pipeline-layout.md -->

Defínelo como `"inline"` cuando el proyecto prefiera chains cortas y compactas.

Defínelo como `"off"` para preservar si cada chain fue escrita en una línea o en varias líneas.

<!-- @include: ../../.fragments/tooling/formatter-pipeline-inline.md -->
