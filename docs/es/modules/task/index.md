---
title: Task
description: Suspende funciones async de tyrus sin bloquear threads del executor.
---

# Task

`tyrus:task` provee cancelación cooperativa, retrasos sin bloqueo, creación de tareas concurrentes, canales tipados entre tareas, combinators para componer varias tareas juntas, un mutex para proteger un valor compartido entre tareas, un reader-writer lock para permitir múltiples lectores concurrentes, y un semaphore para limitar el acceso concurrente a una región de código.

## Imports

<SignatureBlock label="Importaciones">import { cancel, isCancelled, sleep, sleepAsync, spawn, channel, joinAll, race, timeout, Mutex, RwLock, Semaphore, JoinHandle, Sender, Receiver } from "tyrus:task";</SignatureBlock>

## Elementos Exportados

| Elemento | Firma | Propósito |
|----------|-------|-----------|
| [`cancel`](./cancel.md) | `<T>(promise: Promise<T>, reason: string, graceMillis?: u64) => boolean` | Cancela cooperativamente una `Promise` pendiente. |
| [`isCancelled`](./is-cancelled.md) | `() => boolean` | Responde, dentro de una tarea, si le dijeron que parara. |
| [`sleep`](./sleep.md) | `(ms: u64) => void` | Bloquea el hilo actual durante un retraso. |
| [`sleepAsync`](./sleep-async.md) | `(ms: u64) => Promise<void>` | Se resuelve tras un retraso sin bloquear un hilo del executor. |
| [`spawn`](./spawn.md) | `<T>(task: () => Promise<T>) => JoinHandle<T>` | Ejecuta una tarea `async` en paralelo y retorna un handle para ella. |
| [`channel`](./channel.md) | `<T>(capacity?: usize) => [Sender<T>, Receiver<T>]` | Crea un canal tipado entre tareas concurrentes. |
| [`joinAll`](./join-all.md) | `<T>(handles: JoinHandle<T>[]) => Promise<T[]>` | Espera a que cada handle de un array se asiente. |
| [`race`](./race.md) | `<T>(handles: JoinHandle<T>[]) => Promise<T>` | Resuelve con el handle que se asienta primero, cancelando el resto. |
| [`timeout`](./timeout.md) | `<T>(ms: u64, task: Promise<T>) => Promise<T>` | Acota cuánto puede tardar una `Promise`. |
| [`Mutex`](./mutex.md) | `<T>(value: T) => Mutex<T>` | Protege el acceso exclusivo a un valor compartido entre tareas. |
| [`RwLock`](./rwlock.md) | `<T>(value: T) => RwLock<T>` | Permite múltiples lectores concurrentes o un escritor exclusivo sobre un valor compartido. |
| [`Semaphore`](./semaphore.md) | `(permits: usize) => Semaphore` | Limita cuántas tareas entran en una región de código al mismo tiempo. |

`JoinHandle<T>`, el handle que retorna `spawn`, y `Sender<T>`/`Receiver<T>`, el par que retorna `channel`, se exportan desde `tyrus:task`: un binding cuyo tipo solo se infiere no necesita import, pero nombrar uno de ellos en una anotación exige importarlo, como en `import { Sender, channel } from "tyrus:task";`. `Promise<T>` es un tipo nativo y nunca necesita importarse. `MutexGuard<T>`, el propio tipo de retorno de `Mutex<T>.lock()`/`.tryLock()`, `RwReadGuard<T>`/`RwWriteGuard<T>`, los propios tipos de guard de `RwLock<T>`, y `SemaphorePermit`, el propio tipo de retorno de `Semaphore.acquire()`/`.tryAcquire()`, también son tipos nativos.

## El Runtime

Todo ítem aquí corre sobre un runtime asíncrono compartido que no exige configuración. [Runtime](./runtime.md) cubre lo que un proyecto puede decidir sobre él: cuántos hilos de trabajo puede usar y qué pasa con una tarea aún pendiente cuando `main` termina.

## Tasks y Promises

`Promise.*` y `tyrus:task.*` comparten cuatro nombres y no son intercambiables. [Tasks y Promises](./promises.md) cubre cuál usar, y por qué `tyrus:task.race` cancela a los perdedores mientras `Promise.race` no.
