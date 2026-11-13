---
title: Operadores e Expressões
description: Numéricos, lógicos, bitwise, ternário, pipeline, typeof, type assertions, spread, conversão, parsing e process exit.
---

# Operadores e Expressões

Tyrus mantém operadores familiares quando a semântica pode ser checada estaticamente e baixada para código nativo.

## Grupos de Operadores

| Categoria | Operadores |
| --- | --- |
| Aritméticos | `+` `-` `*` `/` `%` `**` |
| Unários | `-` `+` `++` `--` `!` `not` `~` |
| Bitwise | `&` `\|` `^` `~` `<<` `>>` `>>>` |
| Comparação | `>` `<` `>=` `<=` `===` `!==` |
| Lógicos | `&&` `and` `\|\|` `or` |
| Ternário | `? :` |
| Atribuição | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `>>>=` `&&=` `\|\|=` `??=` |
| Null safety | `??` `?.` |
| Borrow | `&` `&mut` |
| Pertencimento | `in` `not in` |
| Checagem de tipo | `is` `not is` |
| Range | `..` `..=` |

`and`, `or` e `not` são equivalentes escritos exatos de `&&`, `||` e `!`, o lexer mapeia cada par para o mesmo token, então as duas grafias podem se misturar livremente na mesma expressão: `active and not suspended or isAdmin` e `active && !suspended || isAdmin` compilam de forma idêntica.

## Comparações Encadeadas

Três ou mais de `<`, `<=`, `>`, `>=`, `===` encadeados diretamente (`a < b < c`) são açúcar sintático para `a < b && b < c`, lendo mais perto da notação algébrica do que o equivalente escrito com `&&`.

<!-- @include: ../../.fragments/language/operators/chained-comparison.md -->

Cada termo na cadeia é avaliado exatamente uma vez, mesmo quando participa de duas comparações, um termo com efeito colateral (uma chamada de função) nunca roda duas vezes. A cadeia também faz curto-circuito como o `&&`: assim que uma comparação falha, os termos seguintes nunca são avaliados. Dois termos (`a < b`) não são afetados por esse açúcar e continuam sendo uma comparação simples; a cadeia só entra a partir de três termos. `!==` nunca encadeia, não existe leitura algébrica para `a != b != c`, então sempre permanece uma comparação simples de dois termos.

## Pipeline

Consulte o [guia do operador pipe](./pipe-operator.md) para conhecer o operador `|>`, o token de topic `%` e a composição de pipelines.

## Ternário

<!-- @include: ../../.fragments/language/operators/ternary.md -->

A condição ternária deve ser `boolean` e os dois branches devem ter o mesmo tipo.

## typeof

O operador `typeof` retorna uma `string` com o nome do tipo tyrus do valor em runtime: o nome do próprio tipo, não uma categoria genérica. Os tipos numéricos continuam distintos em vez de colapsar em `"number"`, um array informa o tipo do elemento, uma interface informa o próprio nome e um opcional mantém o `?`. Só duas formas respondem com uma categoria, por não terem nome a dar: um tipo de objeto anônimo (`"object"`) e uma função (`"function"`).

<!-- @include: ../../.fragments/language/operators/typeof-values.md -->

`typeof` também pode ser usado como type query.

<!-- @include: ../../.fragments/language/operators/typeof-type-query.md -->

Comparações com `typeof` fazem narrowing de valores union no fluxo de controle.

<!-- @include: ../../.fragments/language/operators/typeof-narrowing.md -->

Isto também funciona com enums, `typeof` retorna a representação subjacente:

<!-- @include: ../../.fragments/language/operators/typeof-enum.md -->

## is / not is

O operador `is` verifica se um valor em runtime corresponde a um tipo específico dentro de uma union, retornando `boolean`. Ele é o mecanismo principal para narrowing de union types.

<!-- @include: ../../.fragments/language/operators/is-narrowing.md -->

Dentro do branch `true`, o compilador estreita `value` para `string`. Fora dele, `value` permanece `string | number`.

`not is` é a forma negada:

<!-- @include: ../../.fragments/language/operators/is-not-is.md -->

`is`/`not is` também funcionam em condições de `while` e atribuições booleanas. A checagem roda em runtime comparando a tag de tipo da union contra o tipo alvo.

Um erro capturado mantém o tipo com que foi lançado, então `is` também distingue erros. Em `catch (error)`, `error is ConfigError` é `true` quando o valor lançado era um `ConfigError` (um erro de módulo como `FsError`, ou uma interface que estende `Error`), e dentro desse branch `error` tem todos os campos do subtipo. Um `catch (error: A | B)` estreita para a variante do mesmo jeito. Isso vale também numa função `async`, inclusive para um erro que chega como rejeição de um `await`, e num `catch (error)` que aguarda antes da checagem. Só um `Error` que não veio de um `catch` não consegue distinguir o subtipo (`E3297`). Um handler que confere os erros que conhece termina com `throw error;` para o resto: o valor segue com o seu subtipo, então o próximo `catch` ainda o distingue, e um tipo de erro acrescentado depois é repassado em vez de silenciado.

<!-- @include: ../../.fragments/language/operators/is-caught-error.md -->

## Type Assertions

<!-- @include: ../../.fragments/language/operators/type-assertions.md -->

Uma assertion numérica preserva o valor ou aborta; ela nunca wrappa. As checagens rodam em todos os perfis, então uma execução em debug prevê o que o binário publicado faz. Assertions non-primitive são casts de identidade e devem ser usadas com cuidado.

## Conversão e Parsing

<!-- @include: ../../.fragments/language/operators/conversion-parsing.md -->

Parsing retorna valores opcionais em vez de `NaN`, preservando null safety.

A anotação escolhe a largura do parse, com a largura escolhida pela anotação. `parseInt` estreita para qualquer tipo inteiro e `parseFloat`/`Number` para `f32` ou `f64`; sem anotação, permanecem `i64?` e `f64?`. Um texto cujo valor não cabe no tipo alvo simplesmente não parseia como aquele tipo, então o resultado é ausência em vez de um valor truncado.

<!-- @include: ../../.fragments/language/operators/parse-width.md -->

Um `as T` simples preserva o valor ou aborta; ele nunca dá a volta. O que cabe depende dos limites dos dois tipos, e não da largura deles, então um `u32` guarda um valor acima de `i32::MAX` enquanto um número negativo nunca cabe num `u32`, nem mesmo num da mesma largura.

`as T?` é um cast falível e segue a mesma regra, exceto que um valor fora do intervalo de `T` vira ausência em vez de abortar, e é assim que um programa se recupera disso. Um cast de float para inteiro trunca em direção ao zero, e `NaN` produz ausência.

<!-- @include: ../../.fragments/language/operators/fallible-cast.md -->

O `as` converte entre números; ele nunca reinterpreta um valor como um tipo sem relação. Um enum é lido de volta como o primitivo que o representa, mas nada além disso atravessa: uma string não é um número, e uma interface não é outra nem quando os campos coincidem. Um valor do tipo alvo precisa ser *construído*, e afirmá-lo no lugar disso é um erro.

::: danger error
<!-- @include: ../../.fragments/language/operators/interface-assertion-error.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/operators/validation-error-output.md -->
:::

Isto importa sobretudo onde o tipo carrega validators. Um `User` tem a garantia de satisfazer `@min(18)`, e é essa garantia que permite ao resto do programa dispensar a checagem. Uma conversão que produzisse um `User` sem rodar os validators dele tornaria a garantia uma mentira, então ela é rejeitada: construa o `User`, e trate o `ValidationError` que ele pode lançar.

## Spread e Rest

O operador spread expande arrays e objetos em elementos ou campos individuais:

<!-- @include: ../../.fragments/language/operators/spread.md -->

Parâmetros rest coletam argumentos finais em um array:

<!-- @include: ../../.fragments/language/operators/rest.md -->

## Pertencimento e Ranges

`in` e `not in` checam se um valor pertence a um array, um `Range`, um `Map` ou um `Set`, substituindo chamadas a `.includes()`/`!.includes()` e `.has()`/`!.has()`. `not in` é a forma negada, escrita com a palavra-chave `not` em vez de um operador separado.

`a..b` é um `Range` de `a` até `b`, excluindo `b`; `a..=b` inclui `b`. Um range pode ser anotado como `Range<T>`, como em `const ids: Range<i32> = 1..4;`. Um `Range` tem seu próprio método `.contains(x)`, `x in a..b` e `(a..b).contains(x)` são a mesma checagem.

<!-- @include: ../../.fragments/language/operators/membership-and-ranges.md -->

`in`/`not in` também mantêm seu significado original sobre um objeto simples ou valor de interface: `"campo" in obj` checa se o tipo de `obj` declara um campo chamado `"campo"`, independente da checagem de pertencimento em array/Range/Map/Set acima.
