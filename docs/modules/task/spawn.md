---
title: spawn
description: Run an async task concurrently and get back a JoinHandle to await its result.
---

# spawn

`spawn` runs an `async` task concurrently with its caller and returns a `JoinHandle<T>` for it.

<SignatureBlock label="Signature">spawn&lt;T&gt;(task: () =&gt; Promise&lt;T&gt;): JoinHandle&lt;T&gt;</SignatureBlock>

`JoinHandle<T>.join(): Promise<T>` resolves to the task's result, and `JoinHandle<T>.abort(): void` cooperatively cancels it through the same mechanism as `cancel`. `.join()` consumes the handle, so joining the same handle twice, or calling `.abort()` after `.join()`, is rejected at compile time (`E3033`); `.abort()` itself does not consume the handle, so it may be called any number of times before `.join()`. A thrown error inside a spawned task rejects the `Promise<T>` `join()` returns, exactly like any other async function. Like `Promise<T>`, `JoinHandle<T>` is a native type and never needs to be imported.

`await spawn(...)` directly, and `await spawn(...).join()` chained in the same expression, are both rejected at compile time (`E3031`): both throw away the only way to hold onto the task and do other work while it runs in the background. A discarded `JoinHandle<T>`, `spawn(...);` as a bare statement, never storing the result, is rejected the same way an unused `Promise<T>` is, since its result or rejection could never be observed.

Each call to `tyrus:net`'s `resolve` below is a real network round trip that can take a while. Spawning both lookups lets the second one start before the first has replied, instead of waiting its turn, see the Playground page under Tooling for why this example does not run there:

<!-- @include: ../../.fragments/modules/task/spawn.md -->
