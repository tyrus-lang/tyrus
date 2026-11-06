---
title: watch
description: Observa um diretório por mudanças.
---

# watch

Começa a observar um arquivo ou diretório por mudanças no sistema de arquivos. Retorna um manipulador `FSWatcher` que pode ser usado para parar de observar.

## FSWatcher

O tipo `FSWatcher` tem um único método:

- `close(): void`, para de observar e libera todos os recursos.

<SignatureBlock label="Assinatura">watch(path: string): Result&lt;FSWatcher, FsError&gt;</SignatureBlock>

## FSWatcherEvent

`FSWatcherEvent` descreve um registro de evento com os campos somente para leitura `kind: string` e `path: string`. A API atual de `watch` retorna apenas um manipulador `FSWatcher` e não oferece um método para assinar eventos.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/watch/example.md -->
