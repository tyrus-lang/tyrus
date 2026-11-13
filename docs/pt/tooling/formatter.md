---
title: Formatador
description: Referência completa dos comandos e da configuração do tyrus-fmt.
---

# Formatador

`tyrus fmt` formata arquivos fonte `.tyr` a partir da raiz do projeto. O formatter lê `tyrus-fmt.json` quando ele existe; toda chave é opcional e chaves omitidas usam o default interno.

## Comandos

<!-- @include: ../../.fragments/tooling/formatter-commands.md -->

`tyrus fmt` reescreve arquivos do projeto no local. `--check` verifica formatação sem escrever arquivos e é o modo adequado para CI. `--stdout` imprime a saída formatada para caminhos explícitos. `--init` cria ou atualiza `tyrus-fmt.json` adicionando opções ausentes e preservando valores existentes. `--default` redefine `tyrus-fmt.json` para a configuração default completa.

## Configuração de exemplo

<!-- @include: ../../.fragments/tooling/formatter-example-config.md -->

## Opções

| Opção | Default | Valores | Finalidade |
|-------|---------|---------|------------|
| [`array_expand`](#array_expand) | `"simple"` | `"simple"`, `"expand-all"`, `"off"` | Controla quando arrays literais são expandidos para múltiplas linhas. |
| [`blank_lines`](#blank_lines) | `"collapse"` | `"collapse"`, `"off"` | Controla linhas em branco entre imports, declarações, statements e blocos de fluxo. |
| [`blank_lines_around_blocks`](#blank_lines_around_blocks) | `true` | `true`, `false` | Adiciona linhas em branco antes e depois de statements de bloco e funções. |
| [`blank_lines_around_decorations`](#blank_lines_around_decorations) | `true` | `true`, `false` | Adiciona separação ao redor de campos decorados em interface/type. |
| [`brace_spacing`](#brace_spacing) | `true` | `true`, `false` | Imprime objetos como `{ x: 1 }` ou `{x: 1}`. |
| [`brace_style`](#brace_style) | `"same-line"` | `"same-line"`, `"next-line"`, `"off"` | Controla chaves de abertura em funções e blocos de controle. |
| [`final_newline`](#final_newline) | `true` | `true`, `false` | Garante newline final no arquivo. |
| [`indent_style`](#indent_style) | `"spaces"` | `"spaces"`, `"tab"` | Seleciona espaços ou tabs para indentação. |
| [`indent_width`](#indent_width) | `4` | inteiro `>= 0` | Número de espaços por nível quando `indent_style` é `"spaces"`. |
| [`imports_sort`](#imports_sort) | `true` | `true`, `false` | Ordena imports por caminho de módulo e mantém imports `tyrus:*` primeiro. |
| [`line_width`](#line_width) | `90` | inteiro `>= 0` | Expande arrays, objetos, argumentos de chamada e ternários que excedem a largura configurada. |
| [`object_expand`](#object_expand) | `"simple"` | `"simple"`, `"expand-all"`, `"off"` | Controla quando objetos literais são expandidos para múltiplas linhas. |
| [`pipeline_chain_layout`](#pipeline_chain_layout) | `"line-by-line"` | `"line-by-line"`, `"inline"`, `"off"` | Controla o layout de chains com mais de um `|>`. |
| [`prefer_arrow_function`](#prefer_arrow_function) | `false` | `true`, `false` | Força toda declaração de função de nível superior para a forma arrow (`true`), ou mantém cada uma na forma em que foi escrita, misturando `function` e arrow livremente (`false`). |
| [`quote_style`](#quote_style) | `"double"` | `"double"`, `"single"`, `"off"` | Seleciona aspas preferidas para strings ou preserva as aspas e escapes originais de cada literal. |
| [`remove_unused_imports`](#remove_unused_imports) | `true` | `true`, `false` | Remove specifiers de import não usados ou declarações inteiras de import não usado. |
| [`trailing_comma`](#trailing_comma) | `true` | `true`, `false` | Controla vírgulas finais em arrays e objetos multi-line. |

## `array_expand`

Controla quando arrays literais são expandidos para múltiplas linhas.

<!-- @include: ../../.fragments/tooling/formatter-array-expand.md -->

## `blank_lines`

Controla linhas em branco entre imports, declarações, statements e blocos de controle.

<!-- @include: ../../.fragments/tooling/formatter-blank-lines.md -->

## `blank_lines_around_blocks`

Adiciona linhas em branco antes e depois de statements de bloco e declarações de função.

<!-- @include: ../../.fragments/tooling/formatter-blank-lines-around-blocks.md -->

## `blank_lines_around_decorations`

`blank_lines_around_decorations` mantém interfaces com muitos validators fáceis de ler.

<!-- @include: ../../.fragments/tooling/formatter-decorated-fields.md -->

## `brace_spacing`

Controla se um espaço é impresso entre chaves e seu conteúdo.

<!-- @include: ../../.fragments/tooling/formatter-brace-spacing.md -->

## `brace_style`

Controla a posição da chave de abertura em corpos de função e blocos de controle.

<!-- @include: ../../.fragments/tooling/formatter-brace-style.md -->

## `final_newline`

Controla se uma newline final é adicionada ao final do arquivo.

<!-- @include: ../../.fragments/tooling/formatter-final-newline.md -->

## `indent_style`

Controla o estilo de indentação (espaços ou tabs).

<!-- @include: ../../.fragments/tooling/formatter-indent-style.md -->

## `indent_width`

Controla o número de espaços por nível de indentação.

<!-- @include: ../../.fragments/tooling/formatter-indent-width.md -->

## `imports_sort`

Controla se declarações de import são ordenadas alfabeticamente e agrupadas por tipo de origem.

<!-- @include: ../../.fragments/tooling/formatter-imports-sort.md -->

## `line_width`

Largura máxima de linha. Arrays, objetos, argumentos de chamada e ternários que excedem esse limite são expandidos para múltiplas linhas.

<!-- @include: ../../.fragments/tooling/formatter-line-width.md -->

## `object_expand`

Controla quando objetos literais são expandidos para múltiplas linhas.

<!-- @include: ../../.fragments/tooling/formatter-object-expand.md -->

## `prefer_arrow_function`

Força toda declaração de função de nível superior para a forma arrow ou preserva a forma original.

<!-- @include: ../../.fragments/tooling/formatter-prefer-arrow.md -->

## `quote_style`

Seleciona aspas preferidas para strings. Com `"off"`, o formatter preserva as aspas e escapes originais de cada literal de string.

<!-- @include: ../../.fragments/tooling/formatter-quote-style.md -->

## `remove_unused_imports`

Remove specifiers de import não usados ou declarações inteiras de import não usado.

<!-- @include: ../../.fragments/tooling/formatter-remove-unused.md -->

## `trailing_comma`

Controla vírgulas finais em arrays e objetos multi-line.

<!-- @include: ../../.fragments/tooling/formatter-trailing-comma.md -->

Com `true`, vírgulas finais são adicionadas em arrays e objetos multi-line. Com `false`, nenhuma vírgula final é adicionada.

## Antes e depois

<!-- @include: ../../.fragments/tooling/formatter-before.md -->

<!-- @include: ../../.fragments/tooling/formatter-after.md -->

::: tip Use `--check` no CI
`tyrus fmt --check` verifica formatação sem escrever arquivos, então é o comando certo para pull requests e pipelines de release.
:::

## `pipeline_chain_layout`

`pipeline_chain_layout` é útil para transformações funcionais porque cada etapa continua fácil de escanear.

<!-- @include: ../../.fragments/tooling/formatter-pipeline-layout.md -->

Defina como `"inline"` quando o projeto preferir chains curtas e compactas.

Defina como `"off"` para preservar se cada chain foi escrita em uma linha ou em múltiplas linhas.

<!-- @include: ../../.fragments/tooling/formatter-pipeline-inline.md -->
