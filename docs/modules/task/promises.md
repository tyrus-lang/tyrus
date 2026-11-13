---
title: Tasks and Promises
description: When to reach for Promise.* and when to reach for tyrus:task.*.
---

# Tasks and Promises

`Promise<T>` and `tyrus:task` overlap enough that four of their names look like duplicates, `race` exists on both, and `joinAll` reads like `Promise.all`. They are not duplicates, and picking the wrong one usually costs either cancellation or a compile error.

The short version: a `Promise<T>` is a **value** that will settle, and `Promise.*` combines values you already hold. A `JoinHandle<T>` is a **running task**, and `tyrus:task.*` composes tasks, which means it can also stop them.

## The Two Types

`Promise<T>` arrives from any `async` call. It has `then`/`catch`/`finally`, and nothing about it says who is running the work.

`JoinHandle<T>` arrives only from `spawn`. It names one task running concurrently with the caller. That is what makes `abort()` meaningful; there is nothing to abort about a `Promise<T>` that no task owns.

Both are consumed exactly once. `await pending` takes a `Promise<T>`, and awaiting the same binding again is `E3006`; `await handle.join()` takes a `JoinHandle<T>`, and joining again, or aborting it afterward, is `E3033`. In both cases the value is gone after the first consumption, so bind the result and read that instead.

## Choosing a Combinator

| Goal | Reach for |
|------|-----------|
| Wait for several promises you already hold | [`Promise.all`](../../language/async-await.md) |
| Wait for several promises and keep each failure | `Promise.allSettled` |
| Take the first promise to settle, success or failure | `Promise.race` |
| Take the first promise to *succeed* | `Promise.any` |
| Wait for several spawned tasks | [`joinAll`](./join-all.md) |
| Take the first spawned task to settle, and cancel the rest | [`race`](./race.md) |
| Bound how long one promise may take | [`timeout`](./timeout.md) |
| Stop a pending promise from the outside | [`cancel`](./cancel.md) |

The line between the two `race`s is the important one. `Promise.race` settles with whichever promise settled first and leaves the others running to completion, their work still happens, and its result is simply ignored. `tyrus:task.race` settles the same way and then **cancels every other handle**, because it was given tasks rather than values and a task is something that can be stopped.

## Why `joinAll` Is Not `Promise.all`

`Promise.all` takes `Promise<T>[]`; `joinAll` takes `JoinHandle<T>[]`. They cannot be substituted for each other, and the compiler says so rather than converting silently. `joinAll` also consumes every handle it is given, exactly as `join()` does, so a handle passed to it cannot be joined again afterward.

## Sleeping and Cancelling

`sleepAsync` and `cancel` live in `tyrus:task` rather than on `Promise` because both are about the runtime rather than about a value: `sleepAsync` yields the worker thread instead of holding it, and `cancel` settles a pending promise from outside the task that produced it. `cancel` is the one `tyrus:task` function that does take a `Promise<T>`, cancellation reaches the value, not only the task.
