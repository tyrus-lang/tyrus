---
title: Visão Geral da Sintaxe
description: Uma visão geral curta da sintaxe usada pelo tyrus.
---

# Visão Geral da Sintaxe

tyrus usa sintaxe concisa enquanto preserva as garantias da linguagem. A análise semântica aplica checagens nativas de inicialização, nullability, ownership e fluxo de controle.

## Variáveis

<!-- @include: ../../.fragments/docs/syntax-variables.md -->

Bindings `const` são imutáveis depois da inicialização. Bindings `let` podem ser reatribuídos quando o novo valor é compatível com o tipo declarado.

## Funções

<!-- @include: ../../.fragments/docs/syntax-functions.md -->

Funções usam tipos explícitos para parâmetros e retorno. Uma função que não retorna `void` deve retornar um valor em todo caminho alcançável.

`Result<T, E>` é um valor de tipo que representa sucesso ou erro. Consulte [Valores Result](../../language/types.md#valores-result) para as regras de payload, construtores, narrowing e propagação.

## Objetos

<!-- @include: ../../.fragments/docs/syntax-objects.md -->

Interfaces descrevem formatos de objetos. Literais de objeto são checados estruturalmente contra os campos declarados.

## Módulos

Um módulo publica nomes com `export`, seja na própria declaração, seja por uma cláusula separada que pode renomeá-los. Um nome por módulo pode ser o export default.

<!-- @include: ../../.fragments/docs/syntax/module-exports.md -->

`export protect` estreita quem pode importar um nome. Um nome protegido é visível na pasta que o declara e em todas as subpastas dela, nunca na pasta pai e nunca numa pasta irmã. A regra depende apenas do arquivo declarante, então mover uma pasta inteira preserva todas as relações dentro dela.

<!-- @include: ../../.fragments/docs/syntax/module-protect.md -->

Importá-lo de fora dessa pasta é erro de compilação, e a mensagem nomeia o símbolo, a pasta à qual ele está limitado e as duas saídas.

::: danger error
<!-- @include: ../../.fragments/docs/syntax/module-protect-error.md -->
:::

Uma pasta aninhada é um limite novo e mais estreito: um nome protegido em `src/auth/session/` não é visível em `src/auth/`, nem mesmo através de um arquivo barril no meio. Um re-export carrega o escopo original sem alterá-lo, então `export *` e `export { nome } from` não conseguem alargá-lo, e um import por namespace simplesmente omite os nomes protegidos. `protect` é recusado num export default e num re-export, pois nenhum dos dois declara o nome que publica. Coloque os testes que precisam de um nome protegido dentro da pasta que o possui; não há exceção para arquivos de teste.

O lado do import tem as formas correspondentes. Um import default liga o export default do módulo de origem a qualquer nome local, um import por namespace liga todos os exports de uma vez, e ambos podem ser combinados com imports nomeados em uma única declaração.

<!-- @include: ../../.fragments/docs/syntax/module-imports.md -->

`tyrus.json` pode declarar aliases internos estáticos no objeto `imports`. Cada chave começa com `#` e mapeia uma pasta dentro da raiz do projeto. `#` mapeia a raiz de código fonte, enquanto uma chave mais longa como `#config` mapeia uma pasta aninhada. Prefixos de alias mais longos têm precedência sobre os mais curtos. Aliases são expandidos uma vez em tempo de compilação e nunca resolvem pacotes, URLs ou valores de runtime.

Um namespace é resolvido em tempo de compilação: `math.add(1, 2)` vira uma chamada direta à função daquele módulo, e nenhum valor de namespace existe em runtime. Citar um membro que o módulo não exporta é erro de compilação, e a mensagem nomeia tanto o módulo quanto o membro.

Um módulo também pode encaminhar os exports de outro módulo sem ligá-los localmente.

<!-- @include: ../../.fragments/docs/syntax/module-reexports.md -->

::: warning `export *` nunca encaminha um default
`export *` encaminha apenas exports nomeados. O export default do módulo de origem fica privado. Dois exports publicando o mesmo nome público são erro de compilação, e não um último-vence silencioso.
:::

APIs nativas de runtime são importadas por nomes de módulo estáveis `tyrus:*` quando a superfície está implementada. Imports são resolvidos recursivamente em tempo de compilação; imports circulares, nomes duplicados e imports de símbolos não exportados são erros de compilação.

## Padrões de Segurança

- Todo statement e toda declaração terminam com `;` (`E3218`), incluindo um `const` associado a uma arrow function e uma declaração com destructuring. Um construto que se fecha sozinho não precisa de `;`: `if`, `else`, `while`, `do ... while`, `for`, `for ... of`, `for ... in`, `match`, `switch`, `loop`, `try`/`catch`/`finally`, um bloco `defer`, `interface`, uma behavior `interface`, `function` e `enum`. A cláusula de atualização de um `for` clássico, escrita `i++`, é fechada pelo `)`.
- Variáveis devem ser inicializadas antes do uso.
- A ausência é representada explicitamente por `None` e por tipos opcionais como `string?`; `null` nunca é um valor.
- Valores non-Copy seguem regras de ownership e move.

## Execução Nativa e Cobertura da Linguagem

- APIs de runtime como filesystem e HTTP são recursos nativos linkados ao programa gerado.
- Valores opcionais usam a sintaxe familiar `T?`, `None`, `??` e narrowing; `null` não é um valor e `Optional<T>` é a forma equivalente por extenso.
- Algumas sintaxes são parseadas antes de terem lowering completo para código nativo; páginas experimentais apontam isso explicitamente.
