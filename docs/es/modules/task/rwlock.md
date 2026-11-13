---
title: RwLock
description: Permite múltiples lectores concurrentes o un escritor exclusivo sobre un valor compartido.
---

# RwLock

`RwLock<T>` permite múltiples lectores concurrentes, o un escritor exclusivo, sobre un valor compartido entre tareas generadas concurrentemente.

<SignatureBlock label="Firma">RwLock&lt;T&gt;(value: T): RwLock&lt;T&gt;</SignatureBlock>

`rwlock.read(): Promise<RwReadGuard<T>>` espera acceso de lectura y resuelve con un guard; cualquier número de lectores puede mantener el bloqueo al mismo tiempo. `rwlock.write(): Promise<RwWriteGuard<T>>` espera acceso exclusivo de escritura. `rwlock.tryRead(): RwReadGuard<T>?` y `rwlock.tryWrite(): RwWriteGuard<T>?` nunca esperan: resuelven de inmediato con el guard adquirido, o `None` si el acceso solicitado no está disponible en ese momento.

`guard.value` lee el valor protegido directamente, en el lugar. En un `RwWriteGuard<T>` también escribe, incluyendo asignación compuesta (`guard.value += 1`); en un `RwReadGuard<T>` es de solo lectura, asignar a través de él es un error de compilación. Cualquiera de los dos tipos de guard libera automáticamente su mitad del bloqueo cuando sale de ámbito. Un guard (de cualquier tipo) no puede seguir vivo a través de un `await` (`E3034`).

El payload de `RwLock<T>` está restringido al mismo subconjunto que acepta `Mutex<T>`: `string`, escalares `Copy` y arrays de elementos `Copy` (`E3038` en caso contrario).

<!-- @include: ../../../.fragments/modules/task/rwlock.md -->
