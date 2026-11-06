---
title: Erros e diagnósticos
description: Diagnósticos de compilador, linter, runtime e editor com exemplos.
---

# Erros e diagnósticos

Diagnósticos do tyrus usam códigos estáveis, ranges de origem precisos, labels relacionados e ajuda acionável quando existe uma correção concreta. A mesma análise do compilador alimenta a CLI e o Language Server.

::: tip Leia o range primeiro
Diagnósticos devem apontar para o token, expressão, declaração ou inicializador que causou o problema. Quando o editor mostra um squiggle, comece pelo range destacado.
:::

## Erros do compilador

Erros de tipo interrompem a compilação antes da geração de código LLVM.

::: danger error
<!-- @include: ../../.fragments/language/errors/compiler-error-code.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/errors/compiler-error-output.md -->
:::

## Erros de borrow e move

Valores non-Copy movem por default. Depois de um move, o binding anterior fica inválido.

::: danger error
<!-- @include: ../../.fragments/language/errors/borrow-move-code.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/errors/borrow-move-output.md -->
:::

::: warning Ownership é aplicado antes do runtime
Erros de move e borrow são falhas de compilação. O binário gerado nunca precisa de garbage collector para se recuperar desses erros.
:::

## Warnings do linter

Warnings de lint reportam código válido, mas arriscado, ruidoso ou inconsistente com a arquitetura do projeto.

::: danger error
<!-- @include: ../../.fragments/language/errors/linter-warning-code.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/errors/linter-warning-output.md -->
:::

Quando a correção é determinística, o Language Server expõe a mesma edição como Quick Fix.

## Falhas de runtime

Programas gerados escrevem falhas de runtime em stderr e encerram com status diferente de zero. Saída normal de `print` permanece no stdout.

::: danger error
<!-- @include: ../../.fragments/language/errors/runtime-failure-code.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/errors/runtime-failure-output.md -->
:::

Falhas de runtime incluem checagens de segurança inseridas pelo compilador, como overflow, divisão por zero, narrowing inválido, unwrap forçado de optional e falhas de validação.

## Falhas de validação

Validator decorations podem falhar em tempo de compilação para literais e em runtime para valores dinâmicos.

::: danger error
<!-- @include: ../../.fragments/language/errors/validation-failure-code.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/errors/validation-failure-output.md -->
:::

::: danger Aborts de validação usam stderr
Falhas de validação em runtime chamam o helper de panic de validação do runtime, imprimem o diagnóstico em stderr e encerram o processo com status diferente de zero.
:::

## Diagnósticos no editor

VS Code, Neovim, Zed e IntelliJ/WebStorm devem receber diagnósticos de `tyrus lsp`. Clientes específicos de editor não devem reimplementar regras de tipo.

## Códigos de erro

A tabela lista todos os diagnósticos públicos do compilador no formato `E####`. Use `tyrus explain <código>` para uma referência concisa no terminal; mensagens, labels e Quick Fixes disponíveis continuam dependentes do contexto.

<!-- @include: ../../.fragments/language/errors/error-codes.md -->
