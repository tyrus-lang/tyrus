---
title: Task
description: Suspende funções assíncronas tyrus sem bloquear threads do executor.
---

# Task

`tyrus:task` fornece cancelamento cooperativo, atrasos sem bloqueio, criação de tarefas concorrentes, canais tipados entre tarefas, combinators para compor várias tarefas juntas, um mutex para proteger um valor compartilhado entre tarefas, um reader-writer lock para permitir múltiplos leitores concorrentes, e um semaphore para limitar o acesso concorrente a uma região de código.

## Imports

<SignatureBlock label="Importações">import { cancel, isCancelled, sleep, sleepAsync, spawn, channel, joinAll, race, timeout, Mutex, RwLock, Semaphore, JoinHandle, Sender, Receiver } from "tyrus:task";</SignatureBlock>

## Itens Exportados

| Item | Assinatura | Finalidade |
|------|------------|------------|
| [`cancel`](./cancel.md) | `<T>(promise: Promise<T>, reason: string, graceMillis?: u64) => boolean` | Cancela cooperativamente uma `Promise` pendente. |
| [`isCancelled`](./is-cancelled.md) | `() => boolean` | Responde, dentro de uma tarefa, se ela foi mandada parar. |
| [`sleep`](./sleep.md) | `(ms: u64) => void` | Bloqueia a thread atual por um atraso. |
| [`sleepAsync`](./sleep-async.md) | `(ms: u64) => Promise<void>` | Resolve após um atraso sem bloquear uma thread do executor. |
| [`spawn`](./spawn.md) | `<T>(task: () => Promise<T>) => JoinHandle<T>` | Executa uma tarefa `async` em paralelo e devolve um handle para ela. |
| [`channel`](./channel.md) | `<T>(capacity?: usize) => [Sender<T>, Receiver<T>]` | Cria um canal tipado entre tarefas concorrentes. |
| [`joinAll`](./join-all.md) | `<T>(handles: JoinHandle<T>[]) => Promise<T[]>` | Espera cada handle de um array se estabelecer. |
| [`race`](./race.md) | `<T>(handles: JoinHandle<T>[]) => Promise<T>` | Resolve para o handle que se estabelecer primeiro, cancelando o resto. |
| [`timeout`](./timeout.md) | `<T>(ms: u64, task: Promise<T>) => Promise<T>` | Limita quanto tempo uma `Promise` pode levar. |
| [`Mutex`](./mutex.md) | `<T>(value: T) => Mutex<T>` | Protege acesso exclusivo a um valor compartilhado entre tarefas. |
| [`RwLock`](./rwlock.md) | `<T>(value: T) => RwLock<T>` | Permite múltiplos leitores concorrentes ou um escritor exclusivo sobre um valor compartilhado. |
| [`Semaphore`](./semaphore.md) | `(permits: usize) => Semaphore` | Limita quantas tarefas entram em uma região de código ao mesmo tempo. |

`JoinHandle<T>`, o handle que `spawn` devolve, e `Sender<T>`/`Receiver<T>`, o par que `channel` devolve, são exportados por `tyrus:task`: um binding cujo tipo é só inferido não precisa de import, mas nomear um deles numa anotação exige importá-lo, como em `import { Sender, channel } from "tyrus:task";`. `Promise<T>` é um tipo nativo e nunca precisa ser importado. `MutexGuard<T>`, o próprio tipo de retorno de `Mutex<T>.lock()`/`.tryLock()`, `RwReadGuard<T>`/`RwWriteGuard<T>`, os próprios tipos de guard de `RwLock<T>`, e `SemaphorePermit`, o próprio tipo de retorno de `Semaphore.acquire()`/`.tryAcquire()`, também são tipos nativos.

## O Runtime

Todo item aqui roda sobre um runtime assíncrono compartilhado que não exige configuração. [Runtime](./runtime.md) cobre o que um projeto pode decidir sobre ele: quantas threads de trabalho ele pode usar e o que acontece com uma tarefa ainda pendente quando o `main` termina.

## Tasks e Promises

`Promise.*` e `tyrus:task.*` compartilham quatro nomes e não são intercambiáveis. [Tasks e Promises](./promises.md) cobre qual usar, e por que `tyrus:task.race` cancela os perdedores enquanto `Promise.race` não cancela.
