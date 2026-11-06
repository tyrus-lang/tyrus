---
title: Mutex
description: Protect exclusive access to a shared value across concurrently spawned tasks.
---

# Mutex

`Mutex<T>` protects exclusive access to a value shared between concurrently spawned tasks.

<SignatureBlock label="Signature">Mutex&lt;T&gt;(value: T): Mutex&lt;T&gt;</SignatureBlock>

`mutex.lock(): Promise<MutexGuard<T>>` waits for the lock and resolves with a guard. `mutex.tryLock(): MutexGuard<T>?` never waits: it resolves immediately with the acquired guard, or `None` if the lock is currently held elsewhere.

`guard.value` reads and writes the protected value directly, in place, including compound assignment (`guard.value += 1`). The lock releases automatically once the guard goes out of scope, so there is no explicit unlock call. A `MutexGuard<T>` (or `MutexGuard<T>?` from `tryLock`) must not still be alive across an `await`, since holding a lock across a suspension point is a common source of deadlock and starvation (`E3034`).

`Mutex<T>`'s payload is restricted to the same subset `channel<T>` accepts: `string`, `Copy` scalars, and arrays of `Copy` elements (`E3037` otherwise).

<!-- @include: ../../.fragments/modules/task/mutex.md -->
