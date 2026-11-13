---
title: RwLock
description: Permite múltiplos leitores concorrentes ou um escritor exclusivo sobre um valor compartilhado.
---

# RwLock

`RwLock<T>` permite múltiplos leitores concorrentes, ou um escritor exclusivo, sobre um valor compartilhado entre tarefas geradas concorrentemente.

<SignatureBlock label="Assinatura">RwLock&lt;T&gt;(value: T): RwLock&lt;T&gt;</SignatureBlock>

`rwlock.read(): Promise<RwReadGuard<T>>` aguarda acesso de leitura e resolve com um guard; qualquer número de leitores pode segurar a trava ao mesmo tempo. `rwlock.write(): Promise<RwWriteGuard<T>>` aguarda acesso exclusivo de escrita. `rwlock.tryRead(): RwReadGuard<T>?` e `rwlock.tryWrite(): RwWriteGuard<T>?` nunca aguardam: resolvem imediatamente com o guard adquirido, ou `None` se o acesso solicitado não estiver disponível no momento.

`guard.value` lê o valor protegido diretamente, no lugar. Em um `RwWriteGuard<T>` ele também escreve, incluindo atribuição composta (`guard.value += 1`); em um `RwReadGuard<T>` é somente leitura, atribuir através dele é um erro de compilação. Qualquer um dos dois tipos de guard libera automaticamente sua metade da trava quando sai de escopo. Um guard (de qualquer tipo) não pode continuar vivo através de um `await` (`E3034`).

O payload de `RwLock<T>` é restrito ao mesmo subconjunto que `Mutex<T>` aceita: `string`, scalars `Copy` e arrays de elementos `Copy` (`E3038` caso contrário).

<!-- @include: ../../../.fragments/modules/task/rwlock.md -->
