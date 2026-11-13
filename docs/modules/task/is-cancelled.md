---
title: isCancelled
description: Ask, from inside a task, whether it was told to stop.
---

# isCancelled

`isCancelled` answers whether the running task has been asked to stop.

<SignatureBlock label="Signature">isCancelled(): boolean</SignatureBlock>

It can only be used inside an `async` function, because the answer is about the task that is running.

Cancellation is cooperative: it takes effect at a suspension point. A task spending a long time computing reaches none, and so is never interrupted. That is a consequence of the design rather than a hole in it, and this is what such a loop uses to stop itself.

<!-- @include: ../../.fragments/modules/task/is-cancelled.md -->

Stopping this way is still a cancellation. A value the task returns after seeing the request does not become the Promise's value: work that was already told to stop does not get to report a result.
