---
title: race
description: Resolve or reject with whichever JoinHandle in an array settles first, cancelling the rest.
---

# race

`race` resolves or rejects with whichever `JoinHandle<T>` in an array settles first.

<SignatureBlock label="Signature">race&lt;T&gt;(handles: JoinHandle&lt;T&gt;[]): Promise&lt;T&gt;</SignatureBlock>

Once one handle settles, `race` cooperatively cancels every other handle through the same mechanism `cancel`/`JoinHandle<T>.abort()` use, a handle that has already settled by then (the winner itself, or a loser that happened to finish between the race settling and this cleanup running) is left alone, since cancelling an already-settled Promise is a harmless no-op. Every handle passed to `race` is consumed the same way `.join()` consumes one, so reusing a handle already passed to `race`, or to `joinAll`, or to its own `.join()`, is rejected at compile time (`E3033`).

<!-- @include: ../../.fragments/modules/task/race.md -->
