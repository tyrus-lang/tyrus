---
title: Formatter
description: Complete tyrus-fmt command and configuration reference.
---

# Formatter

`tyrus fmt` formats `.tyr` source files from the project root. The formatter reads `tyrus-fmt.json` when it exists; every key is optional and omitted keys use the built-in default.

## Commands

<!-- @include: ../.fragments/tooling/formatter-commands.md -->

`tyrus fmt` rewrites project files in place. `--check` verifies formatting without writing files and is the CI-friendly mode. `--stdout` prints formatted output for explicit paths. `--init` creates or updates `tyrus-fmt.json` by adding missing options while preserving existing values. `--default` resets `tyrus-fmt.json` to the full default configuration.

## Example Configuration

<!-- @include: ../.fragments/tooling/formatter-example-config.md -->

## Options

| Option | Default | Values | Purpose |
|--------|---------|--------|---------|
| [`array_expand`](#array_expand) | `"simple"` | `"simple"`, `"expand-all"`, `"off"` | Controls when array literals expand to multiple lines. |
| [`blank_lines`](#blank_lines) | `"collapse"` | `"collapse"`, `"off"` | Controls blank lines between imports, declarations, statements, and control-flow blocks. |
| [`blank_lines_around_blocks`](#blank_lines_around_blocks) | `true` | `true`, `false` | Adds blank lines before and after block statements and function declarations. |
| [`blank_lines_around_decorations`](#blank_lines_around_decorations) | `true` | `true`, `false` | Adds separation around decorated interface/type fields. |
| [`brace_spacing`](#brace_spacing) | `true` | `true`, `false` | Prints object literals as `{ x: 1 }` or `{x: 1}`. |
| [`brace_style`](#brace_style) | `"same-line"` | `"same-line"`, `"next-line"`, `"off"` | Controls opening braces for function bodies and control-flow blocks. |
| [`final_newline`](#final_newline) | `true` | `true`, `false` | Ensures the file ends with a trailing newline. |
| [`indent_style`](#indent_style) | `"spaces"` | `"spaces"`, `"tab"` | Selects spaces or tabs for indentation. |
| [`indent_width`](#indent_width) | `4` | integer `>= 0` | Number of spaces per indentation level when `indent_style` is `"spaces"`. |
| [`imports_sort`](#imports_sort) | `true` | `true`, `false` | Sorts imports by module path and keeps `tyrus:*` module imports first. |
| [`line_width`](#line_width) | `90` | integer `>= 0` | Expands arrays, objects, call arguments, and ternaries that exceed the configured width. |
| [`object_expand`](#object_expand) | `"simple"` | `"simple"`, `"expand-all"`, `"off"` | Controls when object literals expand to multiple lines. |
| [`pipeline_chain_layout`](#pipeline_chain_layout) | `"line-by-line"` | `"line-by-line"`, `"inline"`, `"off"` | Controls layout for chains that use more than one `|>`. |
| [`prefer_arrow_function`](#prefer_arrow_function) | `false` | `true`, `false` | Forces every top-level function declaration to the arrow form (`true`), or leaves each one in whichever form it was written, mixing `function` and arrow freely (`false`). |
| [`quote_style`](#quote_style) | `"double"` | `"double"`, `"single"`, `"off"` | Selects preferred string literal quotes or preserves each literal's original quotes and escapes. |
| [`remove_unused_imports`](#remove_unused_imports) | `true` | `true`, `false` | Removes unused import specifiers or entire unused import declarations. |
| [`trailing_comma`](#trailing_comma) | `true` | `true`, `false` | Controls trailing commas in multi-line arrays and objects. |

## `array_expand`

Controls when array literals expand to multiple lines.

<!-- @include: ../.fragments/tooling/formatter-array-expand.md -->

## `blank_lines`

Controls blank lines between imports, declarations, statements, and control-flow blocks.

<!-- @include: ../.fragments/tooling/formatter-blank-lines.md -->

## `blank_lines_around_blocks`

Adds blank lines before and after block statements and function declarations.

<!-- @include: ../.fragments/tooling/formatter-blank-lines-around-blocks.md -->

## `blank_lines_around_decorations`

`blank_lines_around_decorations` keeps validator-heavy interfaces readable.

<!-- @include: ../.fragments/tooling/formatter-decorated-fields.md -->

## `brace_spacing`

Controls whether a space is printed between curly braces and their content.

<!-- @include: ../.fragments/tooling/formatter-brace-spacing.md -->

## `brace_style`

Controls placement of the opening brace for function bodies and control-flow blocks.

<!-- @include: ../.fragments/tooling/formatter-brace-style.md -->

## `final_newline`

Controls whether a trailing newline is added at the end of the file.

<!-- @include: ../.fragments/tooling/formatter-final-newline.md -->

## `indent_style`

Controls indentation style (spaces or tabs).

<!-- @include: ../.fragments/tooling/formatter-indent-style.md -->

## `indent_width`

Controls the number of spaces per indentation level.

<!-- @include: ../.fragments/tooling/formatter-indent-width.md -->

## `imports_sort`

Controls whether import declarations are sorted alphabetically and grouped by source type.

<!-- @include: ../.fragments/tooling/formatter-imports-sort.md -->

## `line_width`

Maximum line length. Arrays, objects, call arguments, and ternaries that exceed this limit expand to multi-line.

<!-- @include: ../.fragments/tooling/formatter-line-width.md -->

## `object_expand`

Controls when object literals expand to multiple lines.

<!-- @include: ../.fragments/tooling/formatter-object-expand.md -->

## `prefer_arrow_function`

Forces every top-level function declaration to the arrow form or preserves the original form.

<!-- @include: ../.fragments/tooling/formatter-prefer-arrow.md -->

## `quote_style`

Selects preferred string literal quotes. With `"off"`, the formatter preserves the original quotes and escapes of every string literal.

<!-- @include: ../.fragments/tooling/formatter-quote-style.md -->

## `remove_unused_imports`

Removes unused import specifiers or entire unused import declarations.

<!-- @include: ../.fragments/tooling/formatter-remove-unused.md -->

## `trailing_comma`

Controls trailing commas in multi-line arrays and objects.

<!-- @include: ../.fragments/tooling/formatter-trailing-comma.md -->

With `true`, trailing commas are added in multi-line arrays and objects. With `false`, no trailing commas are added.

## Before and After

<!-- @include: ../.fragments/tooling/formatter-before.md -->

<!-- @include: ../.fragments/tooling/formatter-after.md -->

::: tip Use `--check` in CI
`tyrus fmt --check` verifies formatting without writing files, which makes it the right command for pull requests and release pipelines.
:::

## `pipeline_chain_layout`

`pipeline_chain_layout` is useful for functional transformations because each step stays easy to scan.

<!-- @include: ../.fragments/tooling/formatter-pipeline-layout.md -->

Set it to `"inline"` when the project prefers compact short chains.

Set it to `"off"` to preserve whether each chain was written inline or across multiple lines.

<!-- @include: ../.fragments/tooling/formatter-pipeline-inline.md -->
