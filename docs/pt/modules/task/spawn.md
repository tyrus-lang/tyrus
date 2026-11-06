---
title: spawn
description: Executa uma tarefa async em paralelo e devolve um JoinHandle para aguardar o resultado.
---

# spawn

`spawn` executa uma tarefa `async` em paralelo com quem a chamou e devolve um `JoinHandle<T>` para ela.

<SignatureBlock label="Assinatura">spawn&lt;T&gt;(task: () =&gt; Promise&lt;T&gt;): JoinHandle&lt;T&gt;</SignatureBlock>

`JoinHandle<T>.join(): Promise<T>` resolve para o resultado da tarefa, e `JoinHandle<T>.abort(): void` cancela cooperativamente a tarefa através do mesmo mecanismo de `cancel`. `.join()` consome o handle, consumindo o handle, então dar join duas vezes no mesmo handle, ou chamar `.abort()` depois de `.join()`, é rejeitado na compilação (`E3033`); `.abort()` em si não consome o handle, então pode ser chamado quantas vezes for antes de `.join()`. Um erro lançado dentro de uma tarefa criada por `spawn` rejeita a `Promise<T>` que `join()` devolve, exatamente como qualquer outra função assíncrona. Assim como `Promise<T>`, `JoinHandle<T>` é um tipo nativo e nunca precisa ser importado.

`await spawn(...)` diretamente, e `await spawn(...).join()` encadeado na mesma expressão, são ambos rejeitados na compilação (`E3031`): os dois descartam a única forma de guardar a tarefa e fazer outro trabalho enquanto ela roda em segundo plano. Um `JoinHandle<T>` descartado, `spawn(...);` como statement solto, sem guardar o retorno, é rejeitado da mesma forma que uma `Promise<T>` não usada, já que o resultado ou a rejeição da tarefa nunca poderiam ser observados.

Cada chamada a `resolve` de `tyrus:net` abaixo é uma ida e volta de rede de verdade, que pode demorar. Disparar as duas buscas com `spawn` deixa a segunda começar antes da primeira responder, em vez de esperar a vez, veja a página do Playground em Tooling para entender por que este exemplo não roda lá:

<!-- @include: ../../../.fragments/modules/task/spawn.md -->
