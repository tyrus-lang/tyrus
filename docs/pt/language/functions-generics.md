---
title: Funções, Closures e Generics
description: Arrow functions, valores de função, closures, funções genéricas, constraints e monomorfização.
---

# Funções, Closures e Generics

Tyrus oferece arrow functions e valores de função nativos.

## Funções

<!-- @include: ../../.fragments/language/functions-generics/functions.md -->

Assinaturas de função incluem tipos de parâmetros e retorno. Corpos de expressão retornam implicitamente. Uma função non-`void` deve retornar um valor em todo caminho alcançável.

## Valores de Função

<!-- @include: ../../.fragments/language/functions-generics/function-values.md -->

Valores de função podem ser armazenados e chamados. Eles podem capturar valores lexicais, sujeitos às regras de ownership e lifetime.

Uma função nomeada é ela mesma um valor de primeira classe: pode ser passada como argumento, atribuída a outro binding e usada como callback de array. Um `const nome = (…) => …` no topo declara uma função nomeada, então o mesmo se aplica a ela.

<!-- @include: ../../.fragments/language/functions-generics/function-values-passing.md -->

Uma função também pode retornar uma função. O valor retornado preserva seu tipo de função, então o binding que o recebe continua chamável sem anotação.

<!-- @include: ../../.fragments/language/functions-generics/make-adder.md -->

A função retornada também pode ser chamada diretamente, sem um binding no meio. O callee passa a ser uma expressão, e não um nome, então não carrega assinatura para consultar: o tipo dele é o contrato inteiro, e chamar algo que não é valor de função dá `E0618` com o range no callee.

<!-- @include: ../../.fragments/language/functions-generics/immediate-call.md -->

::: warning Captura de dois níveis ainda não é suportada
Uma closure que captura um binding de dois níveis acima, como em `(a) => (b) => (c) => a + b + c`, é rejeitada com `E0435`, ancorado na menção que alcança longe demais. Uma closure só pode capturar da função imediatamente ao redor dela.
:::


## Parâmetros Mutáveis

<!-- @include: ../../.fragments/language/functions-generics/mutable-parameters.md -->

`mut` permite reatribuição dentro do corpo da função sem tornar o binding do chamador mutável.

## Parâmetros Rest

<!-- @include: ../../.fragments/language/functions-generics/rest-parameters.md -->

Um parâmetro rest (`...nome: T[]`) precisa ser o último parâmetro e coleta os argumentos finais em um array. Ele aceita argumentos individuais, um `...spread` de um array existente, nenhum argumento, e qualquer combinação junto de parâmetros fixos iniciais. Argumentos individuais são checados contra o tipo do elemento; um argumento spread é checado contra o tipo do array inteiro.

## Closures

<!-- @include: ../../.fragments/language/functions-generics/closures.md -->

Valores de função usam uma representação nativa de closure com ponteiro de função e ponteiro de ambiente. Callbacks síncronos de arrays funcionais podem capturar valores lexicais sem transferir ownership do recurso capturado.

### O Que uma Captura Enxerga

Uma closure que nunca sai da função que a declarou captura os próprios bindings. Leituras enxergam o valor atual, escritas alcançam o binding de fora, e o estado acumula entre chamadas:

<!-- @include: ../../.fragments/language/functions-generics/confined-capture.md -->

Uma closure só permanece dentro da sua função quando toda menção a ela é uma chamada. Retorná-la, passá-la como argumento, apelidá-la com outro nome ou guardá-la em qualquer lugar entrega o valor dela a algo que pode sobreviver à função, e aí os bindings que ela capturou já não existem quando ela roda.

Uma closure assim captura uma *cópia*, tirada no momento em que ela é construída. Ler uma cópia é bem definido. Escrever nela não serve para nada: a escrita cai na cópia, e nada de fora consegue observá-la. Isso é `E0436`, ancorado na própria escrita:

<!-- @include: ../../.fragments/language/functions-generics/escaping-capture.md -->

::: tip
A correção é devolver o valor em vez de escrevê-lo, ou guardar o estado compartilhado num [`Mutex`](../modules/task/mutex.md) que os dois lados alcancem. O segundo é o que uma task iniciada com [`spawn`](../modules/task/spawn.md) precisa, e a mesma regra é o que impede uma task de compartilhar estado mutável com o código que a iniciou.
:::

## Generics

<!-- @include: ../../.fragments/language/functions-generics/generics.md -->

Funções genéricas usam monomorfização nativa: o compilador emite uma especialização LLVM por combinação concreta de tipos. A substituição genérica funciona em arrays, referências, opcionais e tipos de função.
