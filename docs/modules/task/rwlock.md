---
title: RwLock
description: Allow multiple concurrent readers or one exclusive writer over a shared value.
---

# RwLock

`RwLock<T>` allows multiple concurrent readers, or one exclusive writer, over a value shared between concurrently spawned tasks.

<SignatureBlock label="Signature">RwLock&lt;T&gt;(value: T): RwLock&lt;T&gt;</SignatureBlock>

`rwlock.read(): Promise<RwReadGuard<T>>` waits for read access and resolves with a guard; any number of readers may hold the lock at the same time. `rwlock.write(): Promise<RwWriteGuard<T>>` waits for exclusive write access. `rwlock.tryRead(): RwReadGuard<T>?` and `rwlock.tryWrite(): RwWriteGuard<T>?` never wait: they resolve immediately with the acquired guard, or `None` if the requested access is not currently available.

`guard.value` reads the protected value directly, in place. On an `RwWriteGuard<T>` it also writes, including compound assignment (`guard.value += 1`); on an `RwReadGuard<T>` it is read-only, assigning through it is a compile error. Either kind of guard releases its half of the lock automatically once it goes out of scope. A guard (of either kind) must not still be alive across an `await` (`E3034`).

`RwLock<T>`'s payload is restricted to the same subset `Mutex<T>` accepts: `string`, `Copy` scalars, and arrays of `Copy` elements (`E3038` otherwise).

<!-- @include: ../../.fragments/modules/task/rwlock.md -->
