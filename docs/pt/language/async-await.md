---
title: Async e Await
description: Funções async, expressões await, Promise<T>, combinadores e cancelamento cooperativo em tyrus.
---

# Async e Await

tyrus suporta funções `async` executáveis, expressões `await`, valores nativos `Promise<T>` e tratamento estruturado de rejeições.

## Funções Async

Declare uma função async com a palavra-chave `async`. Ela retorna uma `Promise<T>` que resolve para o tipo interno.

<!-- @include: ../../.fragments/language/async-await/async-function.md -->

Uma função async `void` retorna `Promise<void>` e não carrega um payload.

<!-- @include: ../../.fragments/language/async-await/void-async.md -->

## Top-Level Await

Uma `async function main(): Promise<void>` sem parâmetros pode suspender em múltiplas instruções `await` no nível superior.

<!-- @include: ../../.fragments/language/async-await/top-level-await.md -->

## Funções Arrow Async

Funções arrow async seguem a mesma sintaxe.

<!-- @include: ../../.fragments/language/async-await/arrow-async.md -->

## Objetos mutáveis através de await

Um objeto mutável cujos campos obrigatórios armazenados sejam todos valores `Copy` pode permanecer vivo através de suspensões. Seus campos `Copy` podem ser alterados in-place, e o binding inteiro pode ser substituído por outro literal de objeto compatível.

<!-- @include: ../../.fragments/language/async-await/mutable-copy-object.md -->

Objetos mutáveis com campos heap-owned suportados podem ser substituídos integralmente ou atualizados campo a campo. A atribuição a campos string, array, optional e objetos aninhados avalia e clona profundamente o substituto antes de liberar o campo anterior, portanto atribuições derivadas do próprio valor permanecem válidas. Métodos mutáveis de campos de array de strings, como `push`, `splice`, `fill` e `sort`, operam diretamente no campo pertencente ao frame. Cada local recursivamente owned carrega um bit de inicialização, e o drop glue tipado gerado libera somente valores inicializados exatamente uma vez durante substituição ou destruição do frame.

<!-- @include: ../../.fragments/language/async-await/mutable-owned-object.md -->

## Fluxo de controle aninhado

Suspensões podem aparecer através de blocos, ramos condicionais, laços, switches e tratamento estruturado de erros arbitrariamente aninhados. A máquina de estados assíncrona preserva as instâncias de laço ativas e os escopos léxicos entre caminhos irmãos com suspensão, portanto `break`, `continue`, bindings locais de ramo e instruções antes ou depois de cada suspensão mantêm seu comportamento normal. Ambos os ramos de um `if` aninhado podem conter cadeias de suspensão independentes com tamanhos, tipos de payload e bindings diferentes; a condição é avaliada uma única vez.

<!-- @include: ../../.fragments/language/async-await/nested-control-flow.md -->

## Tratamento de Erros

Use `try`/`catch`/`finally` para tratar rejeições async.

Quando um `finally` suspensivo, inclusive um aninhado recursivamente dentro de laços e ramos, é iniciado com um `return`, `break`, `continue` ou `throw` pendente, o frame assíncrono preserva essa completion até o fim do cleanup. Uma completion produzida dentro do `finally` substitui a pendente, e a rejeição de uma operação de cleanup aguardada tem precedência. Cada `finally` executa exatamente uma vez.

::: danger error
<!-- @include: ../../.fragments/language/async-await/error-handling.md -->
:::

Um `throw` dentro de uma função async rejeita a Promise retornada.

::: danger error
<!-- @include: ../../.fragments/language/async-await/throw-async.md -->
:::

## AggregateError

Quando múltiplas operações falham, `AggregateError` agrupa todos os motivos de rejeição.

::: danger error
<!-- @include: ../../.fragments/language/async-await/aggregate-error.md -->
:::

## Promise.resolve e Promise.reject

Crie promises resolvidas ou rejeitadas diretamente.

<SignatureBlock label="Assinatura">Promise.resolve&lt;T&gt;(value: T): Promise&lt;T&gt;</SignatureBlock>

<SignatureBlock label="Assinatura">Promise.reject&lt;T&gt;(reason: Error): Promise&lt;T&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-resolve-reject.md -->

## Promise.all

Aguarda todas as promises resolverem. Rejeita imediatamente se qualquer promise rejeitar.

<SignatureBlock label="Assinatura">Promise.all&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;T[]&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-all.md -->

## Promise.allSettled

Aguarda todas as promises se estabelecerem (resolverem ou rejeitarem). Nunca rejeita.

<SignatureBlock label="Assinatura">Promise.allSettled&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;PromiseSettledResult&lt;T&gt;[]&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-all-settled.md -->

## Promise.race

Resolve ou rejeita com a primeira promise que se estabelecer.

<SignatureBlock label="Assinatura">Promise.race&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;T&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-race.md -->

## Promise.any

Resolve com a primeira promise bem-sucedida. Rejeita apenas se todas as promises rejeitarem.

<SignatureBlock label="Assinatura">Promise.any&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;T&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-any.md -->

## Promise.withResolvers

Crie uma promise com funções externas de resolve e reject.

<SignatureBlock label="Assinatura">Promise.withResolvers&lt;T&gt;(): { promise: Promise&lt;T&gt;, resolve: (value: T) =&gt; void, reject: (reason: Error) =&gt; void }</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-with-resolvers.md -->

## then, catch e finally

Encadeie continuações de promise sem `await`.

<!-- @include: ../../.fragments/language/async-await/then-catch-finally.md -->

## Cancelamento Cooperativo

Use `cancel` de `tyrus:task` para cancelar cooperativamente uma promise pendente.

<!-- @include: ../../.fragments/language/async-await/cancellation.md -->

## Sleep

Atrasa a execução sem bloquear o thread do executor.

<!-- @include: ../../.fragments/language/async-await/sleep.md -->

## Combinadores de Promise na Prática

<!-- @include: ../../.fragments/language/async-await/promise-combinators-practice.md -->

## Aguardando Uma Vez

Uma `Promise<T>` é consumida pelo `await`. O runtime mantém uma única referência a ela, e o `await` toma essa referência e a libera, então aguardar o mesmo binding duas vezes é erro de compilação (`E3006`) e não uma segunda leitura de um valor já resolvido:

<!-- @include: ../../.fragments/language/async-await/await-once.md -->

Guarde o valor aguardado num binding e leia o binding. A mesma regra vale para `JoinHandle<T>` e `.join()`.

## Exemplos de Erro

::: danger Função async deve retornar Promise
Uma função async que retorna um tipo não-Promise é erro de compilação:

<!-- @include: ../../.fragments/language/async-await/err-bad-return.md -->

<!-- @include: ../../.fragments/language/async-await/err-bad-return-output.md -->

:::

::: warning Promise descartada
Uma Promise que não é aguardada, retornada ou armazenada produz um warning:

<!-- @include: ../../.fragments/language/async-await/err-discarded-promise.md -->

<!-- @include: ../../.fragments/language/async-await/err-discarded-promise-output.md -->

:::

::: danger await fora de async
Usar `await` fora de uma função async é erro de compilação:

<!-- @include: ../../.fragments/language/async-await/err-outside-async.md -->

<!-- @include: ../../.fragments/language/async-await/err-outside-async-output.md -->

:::
