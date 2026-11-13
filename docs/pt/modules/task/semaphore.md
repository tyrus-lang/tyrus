---
title: Semaphore
description: Limita quantas tarefas entram em uma região de código ao mesmo tempo.
---

# Semaphore

`Semaphore` limita quantas tarefas podem entrar em uma região de código ao mesmo tempo. Ao contrário de `Mutex<T>`/`RwLock<T>`, ele não protege nenhum valor próprio, apenas conta o acesso concorrente.

<SignatureBlock label="Assinatura">Semaphore(permits: usize): Semaphore</SignatureBlock>

`semaphore.acquire(): Promise<SemaphorePermit>` aguarda uma permissão e resolve assim que uma estiver disponível. `semaphore.tryAcquire(): SemaphorePermit?` nunca aguarda: resolve imediatamente com a permissão adquirida, ou `None` se todas as permissões já estiverem em uso.

A permissão é liberada automaticamente quando sai de escopo, devolvendo sua unidade de concorrência ao semaphore. Ao contrário de um `MutexGuard<T>`/`RwReadGuard<T>`/`RwWriteGuard<T>`, um `SemaphorePermit` pode continuar vivo através de um `await`, segurar uma permissão enquanto se aguarda outro trabalho (limitando I/O concorrente, por exemplo) é o uso comum e pretendido, não um risco a rejeitar.

<!-- @include: ../../../.fragments/modules/task/semaphore.md -->

Uma contagem de permits igual a `0` é erro de compilação (`E3035`): um semaphore sem permits nunca pode ser adquirido, então todo `acquire` esperaria para sempre.
