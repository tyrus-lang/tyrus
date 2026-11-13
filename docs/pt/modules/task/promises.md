---
title: Tasks e Promises
description: Quando usar Promise.* e quando usar tyrus:task.*.
---

# Tasks e Promises

`Promise<T>` e `tyrus:task` se sobrepõem o suficiente para que quatro dos seus nomes pareçam duplicados, `race` existe nos dois, e `joinAll` se lê como `Promise.all`. Eles não são duplicados, e escolher o errado normalmente custa ou o cancelamento, ou um erro de compilação.

A versão curta: uma `Promise<T>` é um **valor** que vai se resolver, e `Promise.*` combina valores que você já tem em mãos. Um `JoinHandle<T>` é uma **tarefa em execução**, e `tyrus:task.*` compõe tarefas, o que significa que também consegue pará-las.

## Os Dois Tipos

`Promise<T>` chega de qualquer chamada `async`. Ela tem `then`/`catch`/`finally`, e nada nela diz quem está executando o trabalho.

`JoinHandle<T>` chega apenas de `spawn`. Ele nomeia uma tarefa rodando concorrentemente com quem a disparou. É isso que dá sentido a `abort()`; não há o que abortar numa `Promise<T>` que nenhuma tarefa possui.

Os dois são consumidos exatamente uma vez. `await pending` consome uma `Promise<T>`, e aguardar o mesmo binding de novo é `E3006`; `await handle.join()` consome um `JoinHandle<T>`, e juntá-lo de novo, ou abortá-lo depois, é `E3033`. Nos dois casos o valor já não existe após o primeiro consumo, então guarde o resultado num binding e leia esse binding.

## Escolhendo um Combinador

| Objetivo | Use |
|------|-----------|
| Esperar várias promises que você já tem | [`Promise.all`](../../language/async-await.md) |
| Esperar várias promises e guardar cada falha | `Promise.allSettled` |
| Pegar a primeira promise a se resolver, com sucesso ou falha | `Promise.race` |
| Pegar a primeira promise a *ter sucesso* | `Promise.any` |
| Esperar várias tarefas disparadas | [`joinAll`](./join-all.md) |
| Pegar a primeira tarefa disparada a se resolver e cancelar as demais | [`race`](./race.md) |
| Limitar quanto tempo uma promise pode levar | [`timeout`](./timeout.md) |
| Parar uma promise pendente de fora | [`cancel`](./cancel.md) |

A diferença entre os dois `race` é a mais importante. `Promise.race` se resolve com a promise que se resolveu primeiro e deixa as outras rodando até o fim, o trabalho delas ainda acontece, e o resultado é simplesmente ignorado. `tyrus:task.race` se resolve da mesma forma e então **cancela todos os outros handles**, porque recebeu tarefas em vez de valores, e uma tarefa é algo que pode ser parado.

## Por Que `joinAll` Não É `Promise.all`

`Promise.all` recebe `Promise<T>[]`; `joinAll` recebe `JoinHandle<T>[]`. Um não substitui o outro, e o compilador diz isso em vez de converter em silêncio. `joinAll` também consome todo handle que recebe, exatamente como `join()` faz, então um handle passado a ele não pode ser juntado de novo depois.

## Dormindo e Cancelando

`sleepAsync` e `cancel` vivem em `tyrus:task` e não em `Promise` porque ambos são sobre o runtime, e não sobre um valor: `sleepAsync` devolve a thread de trabalho em vez de segurá-la, e `cancel` resolve uma promise pendente de fora da tarefa que a produziu. `cancel` é a única função de `tyrus:task` que recebe uma `Promise<T>`, o cancelamento alcança o valor, não só a tarefa.
