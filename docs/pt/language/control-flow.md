---
title: Fluxo de controle
description: Condicionais, loops, switch, match, return, throw e try/catch/finally em tyrus.
---

# Fluxo de controle

Tyrus suporta formas familiares de statements para branches, loops e saídas antecipadas. A sintaxe é concisa, enquanto o checker valida inicialização, ownership e caminhos de retorno.

## Condicionais

<!-- @include: ../../.fragments/language/control-flow/conditionals.md -->

A condição é um `boolean`, ou um `T?` usado como teste de presença: um opcional é verdadeiro quando tem valor. O mesmo vale para `while`, `for`, o ternário `? :`, `&&`, `||` e `!`. No ramo em que o opcional é sabidamente presente, o corpo de um `if`, o lado verdadeiro de um ternário ou o lado direito de um `&&`, ele é estreitado para o tipo não-opcional.

## Condições e statements de curto-circuito

<!-- @include: ../../.fragments/language/control-flow/conditions-and-short-circuit.md -->

`cond && effect();` e `cond || effect();` também são statements válidos: rodam o lado direito pelo efeito de curto-circuito, então o lado direito pode retornar `void`.

## while

<!-- @include: ../../.fragments/language/control-flow/while-loop.md -->

## do...while

<!-- @include: ../../.fragments/language/control-flow/do-while-loop.md -->

`do...while` executa o corpo pelo menos uma vez.

## loop

<!-- @include: ../../.fragments/language/control-flow/loop.md -->

`loop` repete o corpo para sempre até um `break`. Diferente de `while`, nunca avalia uma condição, então é a forma natural para loops que saem no meio do corpo. Quando todo `break` do corpo carrega um valor, o `loop` é uma expressão: ele avalia para o valor passado ao `break` que o encerra, então `loop { break 42; }` tem tipo `i32`. `continue` pula para a próxima repetição. Um `break` com valor só é permitido dentro de uma expressão `loop` (`E0571`); `break` sem valor funciona como em qualquer outro loop.

## for clássico

<!-- @include: ../../.fragments/language/control-flow/classic-for-loop.md -->

Um contador escrito sem anotação assume o tipo do limite do laço: o `i` acima é o `usize` que `length` produz, então ele indexa o array diretamente e não precisa de anotação própria. Um contador medido contra um limite sinalizado continua `i32`, o que mantém válida a aritmética que passa abaixo de zero. Anote o contador quando quiser escolher o tipo você mesmo; a anotação sempre vence.

## for...of

<!-- @include: ../../.fragments/language/control-flow/for-of.md -->

Use `for...of` para arrays e valores iteráveis de runtime. O binding pode desestruturar cada elemento com um padrão de array/tupla `[left, right]` ou de objeto `{ x, y }`, o que combina bem com as entries `[chave, valor]` de um `Map`.

<!-- @include: ../../.fragments/language/control-flow/for-of-destructuring.md -->

`for...of` também itera um `Range` diretamente, associando cada um dos seus valores numéricos por vez, `a..b` exclui `b`, `a..=b` inclui.

<!-- @include: ../../.fragments/language/control-flow/for-of-range.md -->


## for...in

<!-- @include: ../../.fragments/language/control-flow/for-in.md -->

`for...in` itera sobre os nomes dos campos de um objeto. Um array não é um tipo objeto, então use `keys()` com `for...of` quando o índice é o valor que você precisa; índices de array são `usize` na API nativa de coleções.

<!-- @include: ../../.fragments/language/control-flow/for-in-array-keys.md -->

## break e continue

<!-- @include: ../../.fragments/language/control-flow/break-and-continue.md -->

`break` sai do loop ou `switch` envolvente mais próximo. `continue` avança o loop envolvente mais próximo e também funciona dentro de um `switch` ou `match` aninhado. Um arm de `match` nunca tem fallthrough e rejeita um `break` que poderia atingir uma estrutura de controle externa (`E0269`). Um `break` fora de todo loop e `switch`, ou um `continue` fora de todo loop, é um erro (`E0268`).

## return

<!-- @include: ../../.fragments/language/control-flow/return.md -->

O checker rejeita funções com retorno non-`void` que podem terminar sem retornar valor.

## `defer` e `errdefer`

<!-- @include: ../../.fragments/language/control-flow/defer.md -->

`defer` agenda uma expressão ou bloco para rodar quando seu escopo léxico atual termina. `errdefer` tem as mesmas duas formas, mas roda apenas quando o escopo termina por um retorno explícito de erro ou pela propagação de erro com `?`. Ambos são palavras-chave globais da linguagem e não exigem import. Os finalizadores rodam em ordem LIFO; numa saída de erro, os `errdefer` do escopo rodam antes dos `defer`. Escopos aninhados terminam antes do escopo pai.

O corpo captura valores no ponto de registro. Ele não pode usar `return`, `break`, `continue`, `throw`, `await` ou `?`, porque a limpeza não pode substituir a saída em andamento. `errdefer` exige retorno `Result<T, E>` ou `T?`. Os finalizadores são suportados atualmente apenas em funções síncronas. `panic()` aborta imediatamente e não roda nenhum dos finalizadores, portanto limpeza recuperável deve usar um caminho estruturado de erro.

## switch e match

`switch` é uma instrução de fluxo de controle que compara um valor com cases literais. Use-o para dispatch simples sobre valores primitivos ou enums.

<!-- @include: ../../.fragments/language/switch/basic-switch.md -->

::: tip Use switch para dispatch simples
Use `switch` quando o subject é um valor primitivo e cada branch é uma checagem de igualdade literal.
:::

### Fallthrough e break

A execução continua do case correspondente para cada body seguinte. Use `break` para sair do `switch` (ou loop) mais próximo. Omiti-lo é útil quando vários cases compartilham comportamento:

<!-- @include: ../../.fragments/language/switch/fallthrough.md -->

Com `"admin"`, isso imprime `privileged` e `authenticated`. Um `continue` dentro de um `switch` ainda avança o loop envolvente mais próximo. Diferentemente de `switch`, [`match`](./pattern-matching.md) nunca tem fallthrough.

### Cases duplicados

O linter reporta cases repetidos porque o segundo branch é inalcançável.

::: danger error
<!-- @include: ../../.fragments/language/switch/duplicate-case.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/switch/duplicate-case-error.md -->
:::

### Switch vs Match

Use [`match`](./pattern-matching.md) quando o branch precisa de destructuring, guarded arms ou narrowing sobre uma tagged union. Use `switch` para fazer dispatch sobre um enum ou um valor literal.

<!-- @include: ../../.fragments/language/switch/enum-switch.md -->

::: warning Padrões de match não são membros de enum
Um padrão de `match` é um wildcard, um binding, um object pattern ou um literal. Um membro de enum como `PaymentStatus.Paid` não é um padrão, então faça o dispatch sobre um enum com `switch` e reserve o `match` para tagged unions e guarded arms.
:::

## throw e try/catch/finally

<!-- @include: ../../.fragments/language/control-flow/try-catch-finally.md -->

Um `catch` sem tipo vincula `Error`, a interface com `message` e `code` usada pelos erros recuperáveis, então `error.message` e `error.code` ficam disponíveis sem anotação. Uma `string` lançada, ou qualquer outro tipo, continua exigindo o tipo escrito, como em `catch (error: string)`.

<!-- @include: ../../.fragments/language/control-flow/catch-error.md -->

O bloco `finally` executa antes de o controle sair de `try` ou `catch`, inclusive por `return`, `throw`, `break` ou `continue`.

::: warning Evite fluxo inseguro em finally
Retornar, lançar, dar break ou continue em `finally` pode sobrescrever o fluxo original e é reportado por regras de lint.
:::
