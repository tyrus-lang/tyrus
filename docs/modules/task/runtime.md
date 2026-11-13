---
title: Runtime
description: Worker threads, configuration precedence, and what happens to pending tasks when main finishes.
---

# Runtime

Every task a program spawns runs on a shared asynchronous runtime. Nothing has to be started or configured for it to work: the runtime starts the first time a program awaits something, sizes itself to the machine, and stops when the program ends. This page covers the two things a project can decide about it, how much of the machine it may use, and what its ending means for work still in flight.

## Worker Threads

By default the runtime uses one worker thread per logical CPU. A project that wants a different number sets it in `tyrus.json`:

<!-- @include: ../../.fragments/modules/task/runtime/worker-threads.md -->

The value is baked into the compiled program, so it travels with the binary and does not need `tyrus.json` next to it at run time. A count below `1` is a build error: the program would have no way to make progress, and rejecting it at build time is better than shipping a binary that cannot run.

The `TYRUS_WORKER_THREADS` environment variable overrides that value at startup:

<!-- @include: ../../.fragments/modules/task/runtime/worker-threads-env.md -->

Precedence is `TYRUS_WORKER_THREADS`, then `runtime.workerThreads`, then one worker per logical CPU. The environment variable wins so a deployment can fit a program to the machine it actually landed on without rebuilding it.

::: tip
A value well above the number of logical CPUs the runtime detects is honoured, but reported on stderr. Oversubscribing a machine is occasionally deliberate, so this is a warning about a likely mistake rather than a limit the runtime enforces. A value that is not a positive integer is ignored the same way, with a warning, never fatally.
:::

## Shutdown

A program's asynchronous lifetime is `main`'s. When `main` settles, the runtime stops and every task still pending is cancelled rather than awaited, there is no detached task that outlives the program.

<!-- @include: ../../.fragments/modules/task/runtime/abandoned-task.md -->

That program prints `done` and exits. The spawned task is cancelled where it stood. Run it with `TYRUS_REPORT_PENDING_TASKS=1` and the runtime says so on stderr:

<!-- @include: ../../.fragments/modules/task/runtime/abandoned-task-warning.md -->

The way to keep a task alive is to wait for it before returning:

<!-- @include: ../../.fragments/modules/task/runtime/joined-task.md -->

::: tip
The report is opt-in rather than a default warning, because a correct program can end with pending tasks by design: [`race`](./race.md) and [`timeout`](./timeout.md) cancel the tasks they did not pick. Turn it on when a task seems not to have run at all, that is almost always a forgotten `join`. Like every runtime report it goes to stderr, so `stdout` stays exactly what the program printed.
:::

## Loops That Never Yield

A worker thread runs one task at a time. A loop that can neither end nor await keeps its thread for as long as the program lives, and every other task assigned to that thread waits behind it. The compiler warns about the obvious shape (`E3032`): a loop in an `async` function whose condition is the literal `true` and whose body has no `await`, `break`, `return` or `throw`.

<!-- @include: ../../.fragments/modules/task/runtime/busy-wait.md -->

`await sleepAsync(0)` is enough to hand the thread back once per iteration. A loop with a real condition is left alone, even when it turns out to run forever: a lint that fires on ordinary counting loops is one people turn off.
