---
title: Variáveis
description: Declaração de variáveis com let e const, mutabilidade, imutabilidade profunda e bindings opcionais.
---

# Variáveis

O Tyrus fornece duas palavras-chave para declaração de bindings: `const` e `let`. Ambas impõem escopo léxico estrito e participam da checagem estática de tipos, mas diferem em mutabilidade, regras de reatribuição e requisitos de inicialização.

## const vs let

A escolha entre `const` e `let` estabelece se um binding e seus dados subjacentes podem sofrer mutações.

| Recurso | `const` | `let` |
|---|---|---|
| Reatribuição | Proibida em tempo de compilação | Permitida |
| Imutabilidade profunda | Aplicada recursivamente a objetos e arrays | Não aplicada (propriedades e elementos são mutáveis) |
| Inicializador obrigatório | Sempre obrigatório | Opcional (`let x: i32;` é permitido) |
| Shadowing | Proibido | Proibido |

## Bindings imutáveis com const

`const` declara um binding imutável cujo valor não pode ser reatribuído após a inicialização. Use `const` por padrão sempre que uma variável não exigir reatribuição.

<!-- @include: ../../.fragments/language/variables/const-valid.md -->

Tentar atribuir um novo valor a um binding `const` gera erro em tempo de compilação.

::: danger error
<!-- @include: ../../.fragments/language/variables/const-error.md -->
:::

## Imutabilidade profunda com const

No Tyrus, `const` garante imutabilidade profunda. No Tyrus proíbe modificar propriedades de objetos ou elementos de arrays declarados com `const`.

<!-- @include: ../../.fragments/language/variables/deep-immutability-valid.md -->

Modificar um campo de objeto ou elemento de array através de uma referência `const` é rejeitado pelo compilador.

::: danger error
<!-- @include: ../../.fragments/language/variables/deep-immutability-error.md -->
:::

## Bindings mutáveis com let

`let` declara um binding mutável. Ele permite tanto a reatribuição da variável quanto a mutação in-place de estruturas compostas como arrays e objetos.

<!-- @include: ../../.fragments/language/variables/let-valid.md -->

A reatribuição deve continuar respeitando o tipo estaticamente declarado ou inferido da variável.

::: danger error
<!-- @include: ../../.fragments/language/variables/let-error.md -->
:::

## Bindings let sem inicializador

Uma variável declarada com `let` pode ser declarada sem um inicializador imediato, mesmo para tipos não opcionais como `let number: i32;`. Bindings opcionais (`T?`) iniciam vazios e podem ser inspecionados com segurança via coalescência nula (`??`), enquanto bindings não opcionais devem ser atribuídos antes de serem lidos.

<!-- @include: ../../.fragments/language/variables/uninitialized-valid.md -->

Ler uma variável não opcional não inicializada antes da atribuição é um erro, e declarar um `const` sem inicializador é proibido.

::: danger error
<!-- @include: ../../.fragments/language/variables/uninitialized-error.md -->
:::

## Shadowing não é permitido

O Tyrus não permite shadowing de variáveis no mesmo escopo. Para atualizar um valor, declare uma vinculação mutável com `let` e faça a reatribuição em vez de redeclarar a variável.

<!-- @include: ../../.fragments/language/variables/shadowing-valid.md -->

Tentar redeclarar uma variável com `let` ou `const` no mesmo escopo é rejeitado pelo compilador.

::: danger error
<!-- @include: ../../.fragments/language/variables/shadowing-error.md -->
:::

## Inferência de tipos e anotações

Os tipos das variáveis podem ser anotados explicitamente ou inferidos automaticamente a partir da expressão do inicializador.

<!-- @include: ../../.fragments/language/variables/type-annotation-valid.md -->

Quando uma anotação de tipo explícita é fornecida, o valor atribuído deve atender à restrição desse tipo.

::: danger error
<!-- @include: ../../.fragments/language/variables/type-annotation-error.md -->
:::
