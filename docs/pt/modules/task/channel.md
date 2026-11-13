---
title: channel
description: Cria um canal tipado e move valores entre tarefas concorrentes através das pontas Sender e Receiver.
---

# channel

`channel` cria um canal tipado e devolve suas duas pontas próprias (owned) como uma tupla.

<SignatureBlock label="Assinatura">channel&lt;T&gt;(capacity?: usize): [Sender&lt;T&gt;, Receiver&lt;T&gt;]</SignatureBlock>

`capacity` é opcional: omita para um canal ilimitado, ou passe um `usize` para limitar quantos valores ficam bufferizados, `Sender<T>.send`/`trySend` esperam por (ou rejeitam por falta de) espaço quando um canal limitado está cheio. `Sender<T>.send(value: T): Promise<void>` espera por espaço e entrega `value`, rejeitando com `channel is closed` assim que todo `Receiver<T>` for descartado; `Sender<T>.trySend(value: T): boolean` nunca espera, retornando `true` quando aceito ou `false` se o canal está cheio ou fechado. `Receiver<T>.recv(): Promise<T?>` espera pela próxima mensagem, resolvendo para `None` assim que todo `Sender<T>` foi descartado e nada resta bufferizado; `Receiver<T>.tryRecv(): T?` nunca espera, retornando a próxima mensagem bufferizada ou `None` se ainda não houver uma. `Receiver<T>.close(): void` fecha o canal, `send`/`trySend` passam a falhar, enquanto `recv`/`tryRecv` ainda drenam o que já estava bufferizado antes de se estabilizar em `None`. Assim como `Promise<T>` e `JoinHandle<T>`, `Sender<T>`/`Receiver<T>` são tipos nativos e nunca precisam ser importados.

Payloads de canal ficam restritos ao mesmo subconjunto que a própria `Promise<T>` suporta: `string`, arrays de elementos `Copy` e escalares `Copy`, não interfaces arbitrárias.

Um `Sender<T>`/`Receiver<T>` movido para dentro de uma tarefa `spawn` é consumido ali, da mesma forma que qualquer outro valor próprio (owned), capturá-lo duas vezes, digamos, em duas tarefas separadas, é um erro de move em tempo de compilação, então uma única produtora ou consumidora por ponta é o formato a buscar. O exemplo abaixo move `tx` para dentro de um worker gerado (spawned) que devolve um resultado por host assim que ele resolve, deixando o `main` processar os resultados conforme chegam em vez de esperar que toda busca termine, como `Promise.all` faria, veja a página do Playground em Tooling para entender por que este exemplo não roda lá:

<!-- @include: ../../../.fragments/modules/task/channel.md -->

Uma capacidade de `0` é erro de compilação (`E3035`): um canal sem espaço não guarda nada, então todo `send` esperaria por um `recv` que não tem de onde tirar o valor. Omita o argumento para usar a capacidade padrão.
