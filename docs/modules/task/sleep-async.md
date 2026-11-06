---
title: sleepAsync
description: Suspend an async task without blocking its executor thread through tyrus:task.
---

# sleepAsync

`sleepAsync` settles after the requested number of milliseconds without blocking an executor thread.

<SignatureBlock label="Signature">sleepAsync(ms: u64): Promise&lt;void&gt;</SignatureBlock>

Await the returned Promise inside an `async` function. Code below that `await` in the same function waits for the delay, while other ready tasks may continue on the executor thread.

For a blocking delay in synchronous code, use [`sleep`](./sleep.md).

<!-- @include: ../../.fragments/modules/task/sleep-async-example.md -->

::: danger error
<!-- @include: ../../.fragments/modules/task/sleep-async-error.md -->
:::
