---
title: Mutex
description: Protege acesso exclusivo a um valor compartilhado entre tarefas geradas concorrentemente.
---

# Mutex

`Mutex<T>` protege acesso exclusivo a um valor compartilhado entre tarefas geradas concorrentemente.

<SignatureBlock label="Assinatura">Mutex&lt;T&gt;(value: T): Mutex&lt;T&gt;</SignatureBlock>

`mutex.lock(): Promise<MutexGuard<T>>` aguarda a trava e resolve com um guard. `mutex.tryLock(): MutexGuard<T>?` nunca aguarda: resolve imediatamente com o guard adquirido, ou `None` se a trava estiver presa em outro lugar.

`guard.value` lê e escreve o valor protegido diretamente, no lugar, incluindo atribuição composta (`guard.value += 1`). A trava é liberada automaticamente quando o guard sai de escopo, então não existe uma chamada explícita de destravamento. Um `MutexGuard<T>` (ou `MutexGuard<T>?` de `tryLock`) não pode continuar vivo através de um `await`, já que segurar uma trava através de um ponto de suspensão é uma fonte comum de deadlock e starvation (`E3034`).

O payload de `Mutex<T>` é restrito ao mesmo subconjunto que `channel<T>` aceita: `string`, scalars `Copy` e arrays de elementos `Copy` (`E3037` caso contrário).

<!-- @include: ../../../.fragments/modules/task/mutex.md -->
