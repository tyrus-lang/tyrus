---
title: Semaphore
description: Limit how many tasks enter a region of code at the same time.
---

# Semaphore

`Semaphore` limits how many tasks may enter a region of code at the same time. Unlike `Mutex<T>`/`RwLock<T>`, it protects no value of its own, it only counts concurrent access.

<SignatureBlock label="Signature">Semaphore(permits: usize): Semaphore</SignatureBlock>

`semaphore.acquire(): Promise<SemaphorePermit>` waits for a permit and resolves once one is available. `semaphore.tryAcquire(): SemaphorePermit?` never waits: it resolves immediately with the acquired permit, or `None` if every permit is already held.

The permit releases automatically once it goes out of scope, giving its unit of concurrency back to the semaphore. Unlike a `MutexGuard<T>`/`RwReadGuard<T>`/`RwWriteGuard<T>`, a `SemaphorePermit` is allowed to stay alive across an `await`, holding a permit while awaiting other work (bounding concurrent I/O, for example) is the common, intended use, not a hazard to reject.

<!-- @include: ../../.fragments/modules/task/semaphore.md -->

A permit count of `0` is a compile error (`E3035`): a semaphore with no permits can never be acquired, so every `acquire` would wait forever.
