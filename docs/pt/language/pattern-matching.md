---
title: Pattern Matching
description: Expressões match, arms com guard, wildcard, narrowing de unions e exaustividade.
---

# Pattern Matching

`match` é uma forma nativa de fluxo de controle do tyrus. O checker garante que todos os padrões sejam cobertos ou que exista um caso padrão.

Arms usam diretamente `pattern => body`. Para uma única expressão retornada, use a forma compacta `pattern => expression;`; o ponto e vírgula é obrigatório. Arms com bloco continuam suportadas, e seu ponto e vírgula final é opcional. Parênteses ao redor de um padrão são opcionais para todos os padrões suportados, portanto `"ready" =>` e `("ready") =>` são equivalentes.

<!-- @include: ../../.fragments/language/pattern-matching/pattern-matching.md -->

::: warning Um arm compacto é um `return`
`pattern => expression;` retorna `expression` da função que o contém, exatamente como `return expression;`. Por isso, dentro de um laço, ele sai da função inteira, não só do laço, e nada depois do `match` executa. O programa abaixo imprime apenas `one`. Para executar o arm e continuar, use um arm com bloco, `1 => { print("one"); }`.

<!-- @include: ../../.fragments/language/pattern-matching/compact-arm-returns.md -->
:::

## Match em Unions

<!-- @include: ../../.fragments/language/pattern-matching/match-over-unions.md -->

Padrões suportados incluem matches de literal, wildcard `_`, bindings de captura, arms com guard, destructuring de objeto com match de campo literal e padrões de array.

## Padrões de Array

Padrões de array desestruturam valores `Array<T>` e `readonly Array<T>`. `[]` corresponde somente a um array vazio, `[first, second]` corresponde exatamente a dois elementos e `[head, ...tail]` corresponde a um ou mais elementos. Bindings de elementos têm tipo `T`; `tail` é um novo `Array<T>` com posse dos elementos restantes, equivalente a `values.slice(1)`, e não uma view emprestada. Um padrão de array permite no máximo um padrão de resto, ele deve estar no fim e deve ser um binding ou `_`. Elementos literais devem ser atribuíveis a `T`. Padrões de array aninhados e exaustividade por comprimento ainda não estão disponíveis porque arrays comuns não carregam o comprimento no tipo.

<!-- @include: ../../.fragments/language/pattern-matching/array-patterns.md -->

## Match de Literal

Compare diretamente contra valores constantes como strings, números ou booleanos.

<!-- @include: ../../.fragments/language/pattern-matching/literal-match.md -->

## Padrões de Range

Compare um valor numérico ou `char` contra um intervalo limitado: `a..b` exclui `b`, `a..=b` inclui, e qualquer um dos limites pode ser omitido para um range aberto (`..b`, `a..`). Um limite do range deve ser atribuível ao próprio tipo do sujeito, comparar uma `string` contra um range numérico é erro de compilação.

<!-- @include: ../../.fragments/language/pattern-matching/range-pattern.md -->

Um range vazio ou invertido (`10..5`) é um warning, não um erro, ele simplesmente nunca dá match, igual a qualquer outra arm inalcançável. Detectar arms sobrepostas e verificar exaustividade para um tipo numérico finito (`u8`, por exemplo) são decisões deliberadas de escopo, não lacunas: os dois continuam em aberto, como refinamentos rastreados, não comportamento obrigatório.

## Padrões de Binding

Capture valores correspondidos em variáveis para usar no corpo da arm.

<!-- @include: ../../.fragments/language/pattern-matching/binding-pattern.md -->

## Padrão Wildcard

Use `_` para corresponder a qualquer valor que não foi correspondido por arms anteriores.

<!-- @include: ../../.fragments/language/pattern-matching/wildcard-pattern.md -->

## Arms com Guard

Adicione condições `if` em arms para correspondência mais específica.

<!-- @include: ../../.fragments/language/pattern-matching/guard-pattern.md -->

## Destructuring de Objeto

Desestruture objetos e corresponda a valores específicos de campos.

<!-- @include: ../../.fragments/language/pattern-matching/object-destructure.md -->

## Padrões Combinados

Combine narrowing de tipo, guards e match de literal para lógica complexa.

<!-- @include: ../../.fragments/language/pattern-matching/combined-patterns.md -->

## Padrões de Variante de Enum

Faça match de um enum algébrico por variante e ligue um nome para cada campo do payload. Uma variante com dois campos exige dois bindings: um único nome para um payload de vários campos é recusado, pois não há como ler os campos de volta a partir dele. Uma variante sem payload não aceita binding algum.

<!-- @include: ../../.fragments/language/pattern-matching/enum-variant-pattern.md -->

## Exaustividade

Para subjects fechados como enum, boolean, literal union e tagged union, `match` deve cobrir toda variante ou incluir `_`. Arms inalcançáveis são reportados como warnings.

::: tip Switch está documentado em Fluxo de controle
Use [`switch`](./control-flow.md#switch-e-match) para dispatch literal sobre valores primitivos. Use `match` quando destructuring, narrowing ou exaustividade importam.
:::
