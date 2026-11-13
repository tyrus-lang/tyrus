---
title: cancel
description: Ask a task to stop, and wait for it to finish stopping.
---

# cancel

`cancel` asks a pending Promise to stop. The work unwinds through its cleanup and the Promise settles as cancelled.

<SignatureBlock label="Signature">cancel&lt;T&gt;(promise: Promise&lt;T&gt;, reason: string, graceMillis?: u64): boolean</SignatureBlock>

It returns `true` only for the call that starts the cancellation, and `false` for one arriving after the Promise already settled or was already asked to stop.

Cancellation is cooperative: it takes effect at a suspension point, never in the middle of a statement. A task in the middle of a long computation reaches no suspension point and so is not interrupted; [`isCancelled`](./is-cancelled.md) is what such a loop uses to stop itself.

## What runs, and what does not

A cancelled task runs its `finally` blocks and then leaves. Whatever it holds across an `await` is released, and the statements after the block do not run.

A `catch` does not see the cancellation. A `catch` is written for the failures a program expects from its own work, and handling "you were told to stop" as one of them means a handler that logs and carries on, or retries, starts fresh work on a task that is already ending.

Being cancelled and awaiting something cancelled are different things, and only the first skips the `catch`. A task that awaits a Promise someone else cancelled has not been told anything, and its `catch` is the only place it can learn what happened.

<!-- @include: ../../.fragments/modules/task/cancel.md -->

## When the Promise settles

Once the work has finished unwinding, not when the request is made. A caller that stops a worker and then reuses its port, its file or its connection is otherwise racing the release, which shows up as a rare failure rather than a wrong answer.

The reason the caller passed is what the Promise carries, whatever the task reached on the way out: a value it managed to return is the product of work already asked to stop.

Cancellation reaches work the runtime is doing on the task's behalf. A task waiting on a request has that request stopped, and the connection closes.

## The deadline

`graceMillis` bounds how long the cleanup may take. Past it the work is cut, and the Promise carries `cancellation cleanup exceeded its deadline` instead of the original reason, because a caller cannot otherwise tell a clean stop from one that abandoned work half-released.

Omitting it means the cleanup is never cut. Cleanup still making progress is not something to interrupt on a guess, so a caller with somewhere to be is the one that says how long it will wait.
