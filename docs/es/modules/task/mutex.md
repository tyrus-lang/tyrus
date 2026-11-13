---
title: Mutex
description: Protege el acceso exclusivo a un valor compartido entre tareas generadas concurrentemente.
---

# Mutex

`Mutex<T>` protege el acceso exclusivo a un valor compartido entre tareas generadas concurrentemente.

<SignatureBlock label="Firma">Mutex&lt;T&gt;(value: T): Mutex&lt;T&gt;</SignatureBlock>

`mutex.lock(): Promise<MutexGuard<T>>` espera el bloqueo y resuelve con un guard. `mutex.tryLock(): MutexGuard<T>?` nunca espera: resuelve de inmediato con el guard adquirido, o `None` si el bloqueo está en manos de otra parte.

`guard.value` lee y escribe el valor protegido directamente, en el lugar, incluyendo asignación compuesta (`guard.value += 1`). El bloqueo se libera automáticamente cuando el guard sale de ámbito, así que no existe una llamada explícita de desbloqueo. Un `MutexGuard<T>` (o `MutexGuard<T>?` de `tryLock`) no puede seguir vivo a través de un `await`, ya que mantener un bloqueo a través de un punto de suspensión es una fuente común de deadlock e inanición (`E3034`).

El payload de `Mutex<T>` está restringido al mismo subconjunto que acepta `channel<T>`: `string`, escalares `Copy` y arrays de elementos `Copy` (`E3037` en caso contrario).

<!-- @include: ../../../.fragments/modules/task/mutex.md -->
