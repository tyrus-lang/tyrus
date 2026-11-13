---
title: VS Code
description: Experiência de edição do tyrus no VS Code com diagnósticos, completion, hover, inlay hints, semantic tokens, Quick Fixes e code lenses de Quick Fix.
---

# VS Code

A extensão do VS Code é um cliente para o servidor compartilhado `tyrus lsp`. Ela deve parecer uma experiência nativa de linguagem mantendo regras semânticas no compilador e no Language Server.

## Instalar e abrir

Faça build ou instale a extensão a partir de `editors/vscode`, depois abra um projeto que contém arquivos `.tyr`.

Instale a [extensão Tyrus publicada no Marketplace do VS Code](https://marketplace.visualstudio.com/items?itemName=tyrus-lang.tyrus).

<!-- @include: ../../.fragments/tooling/install.md -->

A extensão se seleciona automaticamente como formatter padrão para arquivos `.tyr` e habilita a formatação ao salvar. Esses defaults não adicionam entradas ao `settings.json`, e configurações explícitas do usuário ou workspace têm precedência. Abra Settings com `Ctrl+,` e pesquise `tyrus`, ou pesquise `@ext:tyrus.tyr-language-support` para mostrar apenas as configurações da extensão.

O logotipo do tyrus identifica a extensão na tela Extensions, e arquivos `.tyr` expõem o ícone da linguagem tyrus no Explorer. Um tema de ícones de arquivo de terceiros pode substituir intencionalmente ícones fornecidos pelas linguagens pela própria arte.

::: tip Inteligência compartilhada
VS Code, Neovim, Zed e IntelliJ/WebStorm devem receber os mesmos diagnósticos e code intelligence de `tyrus lsp`.
:::

## Diagnósticos

Diagnósticos do compilador e do linter aparecem como squiggles no editor e no painel Problems. Por padrão, a extensão também adiciona um fundo translúcido à linha inteira: vermelho para erros e amarelo para warnings. O squiggle e seu range preciso de origem permanecem visíveis.

Defina `tyrus.diagnostics.highlightWholeLine` como `false` no `settings.json` para desativar o fundo da linha inteira sem alterar os diagnósticos. Como alternativa, pressione `Ctrl+,`, pesquise `tyrus`, localize **Tyrus › Diagnostics: Highlight Whole Line** e desmarque a caixa.

::: danger error
<!-- @include: ../../.fragments/tooling/diagnostics.md -->
:::

Feedback esperado no editor:

| Origem | Severidade | Exemplo |
|--------|------------|---------|
| Compilador | Error | `expected i32, found string` |
| Linter | Warning | `retries is never reassigned` |
| Schema de formatter/lint config | Error ou warning | Chaves desconhecidas ou inválidas em `tyrus-fmt.json` e `tyrus-lint.json`. |

## Completion

Completion inclui símbolos locais, símbolos importados, exports de módulos nativos, keywords, tipos primitivos, validator decorators, campos de objeto, chaves de opção de rota como `handler`, `response`, `params` e `config`, chaves aninhadas de rate limit como `max`, `timeWindow` e `globalMax`, e membros de arrays, strings, `Regex`, `HttpRouter` e `HttpStatus`. Hover em uma opção de rota explica seu valor esperado e sua finalidade.

<!-- @include: ../../.fragments/tooling/completion.md -->

## Hover e Signature Help

Hover mostra tipos inferidos, assinaturas de imports nativos, campos de interface, documentação de validators e diagnósticos. Signature help mostra nomes de parâmetros e o argumento ativo durante o preenchimento de uma chamada.

<!-- @include: ../../.fragments/tooling/hover.md -->

::: details Exemplo de hover
<!-- @include: ../../.fragments/tooling/hover-example.md -->
:::

## Inlay hints

O Language Server fornece inlay hints de nomes de parâmetros e tipos inferidos.

<!-- @include: ../../.fragments/tooling/inlay-hints-source.md -->

Hints esperados:

<!-- @include: ../../.fragments/tooling/inlay-hints-expected.md -->

## Quick Fixes

Quick Fixes ficam disponíveis quando a edição é determinística.

<!-- @include: ../../.fragments/tooling/quick-fix.md -->

Exemplos incluem importar um símbolo ausente, remover imports não usados, aplicar `prefer-const` e tomar emprestado um valor que seria movido.

## Code Lenses de Quick Fix

Quando um diagnóstico em `.tyr` oferece pelo menos um Quick Fix determinístico, o Language Server também o expõe como um code lens acima da linha com o problema. O lens mostra exatamente o título do Quick Fix (por exemplo, `Change let to const`), e clicar nele aplica a edição imediatamente, sem abrir o menu de Quick Fix. Quando um diagnóstico tem mais de uma correção, cada correção ganha o próprio lens ao lado dos outros. Os Quick Fixes de arquivos `.env` continuam disponíveis somente pela lâmpada.

O Quick Fix normal continua funcionando pela lâmpada e pelos comandos padrão do editor; os lenses apenas adicionam um ponto de entrada mais próximo do código.

Defina `tyrus.codeLens.quickFixes.enable` como `false` no `settings.json` para esconder os code lenses de Quick Fix sem alterar os próprios Quick Fixes. A mudança nesta configuração vale depois de recarregar a janela.

## Semantic highlighting

Semantic tokens melhoram a coloração de símbolos, tipos primitivos, tipos nativos, interfaces, type aliases, keywords, literal keywords e palavras reservadas para recursos futuros da linguagem.

::: warning Highlighting não é validação semântica
Cores de sintaxe ajudam a leitura, mas diagnósticos do compilador e do linter são a fonte de verdade para correção.
:::
