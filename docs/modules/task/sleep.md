---
title: sleep
description: Block the current thread through tyrus:task.
---

# sleep

`sleep` blocks the current thread for the requested number of milliseconds.

<SignatureBlock label="Signature">sleep(ms: u64): void</SignatureBlock>

Use `sleep` in command-line programs, simple scripts, and tests where blocking the current thread is intended. In an async program with one worker thread, it blocks the entire runtime.

## What continues while waiting

Code below an `await sleepAsync(...)` in the same function waits for the delay. Other ready tasks may run on that thread while the current function is suspended. In the first program, `sleep` blocks the thread, so it prints `first` and then `second`. In the second program, `sleepAsync` suspends the first task, so it prints `second` and then `first`.

<!-- @include: ../../.fragments/modules/task/sleep-ordering.md -->

::: danger error
<!-- @include: ../../.fragments/modules/task/sleep-error.md -->
:::
