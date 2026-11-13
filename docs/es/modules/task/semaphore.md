---
title: Semaphore
description: Limita cuántas tareas entran en una región de código al mismo tiempo.
---

# Semaphore

`Semaphore` limita cuántas tareas pueden entrar en una región de código al mismo tiempo. A diferencia de `Mutex<T>`/`RwLock<T>`, no protege ningún valor propio, solo cuenta el acceso concurrente.

<SignatureBlock label="Firma">Semaphore(permits: usize): Semaphore</SignatureBlock>

`semaphore.acquire(): Promise<SemaphorePermit>` espera un permiso y se resuelve en cuanto uno está disponible. `semaphore.tryAcquire(): SemaphorePermit?` nunca espera: se resuelve de inmediato con el permiso adquirido, o `None` si todos los permisos ya están en uso.

El permiso se libera automáticamente cuando sale de ámbito, devolviendo su unidad de concurrencia al semaphore. A diferencia de un `MutexGuard<T>`/`RwReadGuard<T>`/`RwWriteGuard<T>`, un `SemaphorePermit` puede seguir vivo a través de un `await`, mantener un permiso mientras se espera otro trabajo (acotando E/S concurrente, por ejemplo) es el uso común y deseado, no un riesgo a rechazar.

<!-- @include: ../../../.fragments/modules/task/semaphore.md -->

Una cuenta de permits igual a `0` es un error de compilación (`E3035`): un semaphore sin permits nunca puede adquirirse, así que todo `acquire` esperaría para siempre.
