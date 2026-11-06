---
title: Task
description: Suspend asynchronous tyrus functions without blocking executor threads.
---

# Task

`tyrus:task` provides cooperative cancellation, non-blocking delays, concurrent task spawning, typed channels between tasks, combinators for composing multiple tasks together, a mutex for protecting a value shared between tasks, a reader-writer lock for allowing multiple concurrent readers, and a semaphore for limiting concurrent access to a region of code.

## Imports

<SignatureBlock label="Imports">import { cancel, isCancelled, sleep, sleepAsync, spawn, channel, joinAll, race, timeout, Mutex, RwLock, Semaphore, JoinHandle, Sender, Receiver } from "tyrus:task";</SignatureBlock>

## Exported Items

| Item | Signature | Purpose |
|------|-----------|---------|
| [`cancel`](./cancel.md) | `<T>(promise: Promise<T>, reason: string, graceMillis?: u64) => boolean` | Cooperatively cancels a pending `Promise`. |
| [`isCancelled`](./is-cancelled.md) | `() => boolean` | Answers, inside a task, whether it was told to stop. |
| [`sleep`](./sleep.md) | `(ms: u64) => void` | Blocks the current thread for a delay. |
| [`sleepAsync`](./sleep-async.md) | `(ms: u64) => Promise<void>` | Settles after a delay without blocking an executor thread. |
| [`spawn`](./spawn.md) | `<T>(task: () => Promise<T>) => JoinHandle<T>` | Runs an `async` task concurrently and returns a handle to it. |
| [`channel`](./channel.md) | `<T>(capacity?: usize) => [Sender<T>, Receiver<T>]` | Creates a typed channel between concurrent tasks. |
| [`joinAll`](./join-all.md) | `<T>(handles: JoinHandle<T>[]) => Promise<T[]>` | Waits for every handle in an array to settle. |
| [`race`](./race.md) | `<T>(handles: JoinHandle<T>[]) => Promise<T>` | Resolves with whichever handle settles first, cancelling the rest. |
| [`timeout`](./timeout.md) | `<T>(ms: u64, task: Promise<T>) => Promise<T>` | Bounds how long a `Promise` is allowed to take. |
| [`Mutex`](./mutex.md) | `<T>(value: T) => Mutex<T>` | Protects exclusive access to a value shared between tasks. |
| [`RwLock`](./rwlock.md) | `<T>(value: T) => RwLock<T>` | Allows multiple concurrent readers or one exclusive writer over a shared value. |
| [`Semaphore`](./semaphore.md) | `(permits: usize) => Semaphore` | Limits how many tasks enter a region of code at the same time. |

`JoinHandle<T>`, the handle `spawn` returns, and `Sender<T>`/`Receiver<T>`, the pair `channel` returns, are exported by `tyrus:task`: a binding whose type is only inferred needs no import, but naming one of them in an annotation requires importing it, as in `import { Sender, channel } from "tyrus:task";`. `Promise<T>` is a native type and never needs to be imported. `MutexGuard<T>`, `Mutex<T>.lock()`/`.tryLock()`'s own return type, `RwReadGuard<T>`/`RwWriteGuard<T>`, `RwLock<T>`'s own guard types, and `SemaphorePermit`, `Semaphore.acquire()`/`.tryAcquire()`'s own return type, are native types too.

## The Runtime

Every item here runs on a shared asynchronous runtime that needs no setup. [Runtime](./runtime.md) covers what a project can decide about it: how many worker threads it may use, and what happens to a task still pending when `main` finishes.

## Tasks and Promises

`Promise.*` and `tyrus:task.*` share four names and are not interchangeable. [Tasks and Promises](./promises.md) covers which to reach for, and why `tyrus:task.race` cancels the losers while `Promise.race` does not.
