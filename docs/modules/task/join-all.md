---
title: joinAll
description: Wait for every JoinHandle in an array to settle, keeping their results in order.
---

# joinAll

`joinAll` waits for every `JoinHandle<T>` in an array to settle, resolving to their results in the same order the handles were given.

<SignatureBlock label="Signature">joinAll&lt;T&gt;(handles: JoinHandle&lt;T&gt;[]): Promise&lt;T[]&gt;</SignatureBlock>

It resolves once every handle has, regardless of which one actually finished first, `results[0]` is always `handles[0]`'s own result, not whichever settled earliest. It rejects with the first rejection it observes, the same way `Promise.all` does. Every handle passed to `joinAll` is consumed the same way `.join()` consumes one, so reusing a handle already passed to `joinAll`, or to `race`, or to its own `.join()`, is rejected at compile time (`E3033`).

<!-- @include: ../../.fragments/modules/task/join-all.md -->
