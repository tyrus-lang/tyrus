---
title: Readonly
description: As seis formas de readonly, propriedades de interface, posição de tipo, parâmetros, Readonly<T>, as const e readonly const.
---

# Readonly

`readonly` trata de *mutabilidade*: diz que não se pode escrever através de um valor, em nenhuma profundidade. Não trata de *rebinding*, que é o que `const` e `let` decidem, nem de *estreitar um tipo para um literal*, que é o que o `as const` acrescenta por cima.

A palavra-chave aparece em seis lugares distintos. Todos produzem a mesma garantia, nada atrás deste valor pode receber atribuição, e diferem apenas naquilo a que se aplicam.

| Forma | Onde fica | O que protege |
|---|---|---|
| `readonly id: i32` | propriedade de interface | aquela propriedade, e tudo abaixo dela |
| `readonly Config` | posição de tipo | todo valor daquele tipo |
| `config: readonly Config` | parâmetro | o argumento, enquanto durar a chamada |
| `Readonly<Config>` | tipo utilitário | todas as propriedades de uma interface de uma vez |
| `[1, 2] as const` | sufixo de expressão | o valor, e estreita cada elemento para o seu tipo literal |
| `readonly const PRIMES: i32[]` | declaração | o binding, profundamente |

A garantia é recursiva em todas elas. Um valor `readonly` um nível abaixo continua `readonly`, então uma escrita aninhada é recusada exatamente como a escrita de superfície.

## `readonly` em propriedade de interface

A forma mais estreita: uma propriedade de uma interface. Todo o resto da interface continua gravável.

::: danger error
<!-- @include: ../../.fragments/language/readonly/interface-property.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/interface-property-diagnostic.md -->
:::

## `readonly` em posição de tipo

Escrito antes de um tipo, `readonly` produz a visão somente leitura dele. Num array, isso remove os métodos que mutam.

::: danger error
<!-- @include: ../../.fragments/language/readonly/type-position.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/type-position-diagnostic.md -->
:::

Um valor readonly também não pode ser lavado para dentro de um binding mutável, o que devolveria justamente o acesso de escrita que o tipo recusou.

::: danger error
<!-- @include: ../../.fragments/language/readonly/type-position-transfer.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/type-position-transfer-diagnostic.md -->
:::

## `readonly` em parâmetro

Um parâmetro recebe `readonly` na posição do *tipo*, não antes do nome. `readonly config: Config` é rejeitado; `config: readonly Config` é a forma.

<!-- @include: ../../.fragments/language/readonly/parameter.md -->

Esta é a assinatura a escolher quando uma função lê um valor que não criou: quem chama enxerga só pela assinatura que o seu valor volta intacto.

::: danger error
<!-- @include: ../../.fragments/language/readonly/parameter-mutation.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/parameter-mutation-diagnostic.md -->
:::

## `Readonly<T>`

`Readonly<T>` aplica o qualificador a todas as propriedades de uma interface de uma vez, em vez de uma por uma. É recursivo. A descrição completa, com os casos aninhado e de array, está na página [Objetos e interfaces](/pt/language/objects#readonly-t).

## `as const`

`as const` faz duas coisas, e só uma delas é sobre mutabilidade. Congela o valor e estreita cada elemento para o seu próprio tipo literal: em `[3000, 3001] as const` o primeiro elemento tem o tipo `3000`, não `i32`.

<!-- @include: ../../.fragments/language/readonly/as-const.md -->

::: tip `as const` não substitui `readonly`
Use `readonly` quando o ponto é que nada pode ser escrito. Use `as const` quando o ponto é que os *valores literais exatos* fazem parte do tipo. O congelamento que vem junto com `as const` é consequência desse estreitamento, não a sua finalidade.
:::

## `readonly const`

`readonly const` é a forma de declaração. O binding nunca é reatribuído, porque é um `const`, e nada atrás dele pode ser escrito, porque é `readonly`.

<!-- @include: ../../.fragments/language/readonly/readonly-const.md -->

Um `readonly const` global cujo inicializador é uma tabela literal não custa nada em tempo de execução: é emitido em memória somente leitura, sem alocação, sem stores de elemento e sem nada a liberar.

`readonly const x: T` e `const x: readonly T` são a mesma declaração, então o formatador fixa a primeira grafia e escreve as duas assim. O único lugar em que não faz isso é o inicializador de um `for`, onde a gramática só admite `const` e `let`.

A palavra-chave vem antes de `const`, e a ordem é verificada.

::: danger error
<!-- @include: ../../.fragments/language/readonly/readonly-const-order.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/readonly-const-order-diagnostic.md -->
:::

A anotação é obrigatória, porque não há mais nada a que o qualificador possa se prender.

::: danger error
<!-- @include: ../../.fragments/language/readonly/readonly-const-annotation.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/readonly-const-annotation-diagnostic.md -->
:::

`readonly` qualifica apenas `const`. Um `let` pode ser reatribuído, então um `let` readonly prometeria algo que a linha seguinte poderia tirar, e é rejeitado.

## `readonly` não é `const`

Os dois são fáceis de confundir porque um binding `const` já é profundamente imutável no tyrus. A diferença aparece assim que o valor viaja.

`const` é uma propriedade do *binding*: impede que aquele nome seja reatribuído e impede escritas através daquele nome. Não diz nada sobre o tipo, então passar o valor para uma função que recebe um parâmetro mutável é permitido.

`readonly` é uma propriedade do *tipo*: viaja com o valor para parâmetros, tipos de retorno, campos e outros bindings. É isso que o torna utilizável numa assinatura, e é isso que diferencia um `readonly const` de um `const` comum, a proteção sobrevive à entrega do valor.
