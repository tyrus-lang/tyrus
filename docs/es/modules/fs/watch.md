---
title: watch
description: Observa un directorio por cambios.
---

# watch

Comienza a observar un archivo o directorio por cambios en el sistema de archivos. Devuelve un manejador `FSWatcher` que se puede usar para dejar de observar.

## FSWatcher

El tipo `FSWatcher` tiene un único método:

- `close(): void`, deja de observar y libera todos los recursos.

<SignatureBlock label="Firma">watch(path: string): Result&lt;FSWatcher, FsError&gt;</SignatureBlock>

## FSWatcherEvent

`FSWatcherEvent` describe un registro de evento con los campos de solo lectura `kind: string` y `path: string`. La API actual de `watch` solo devuelve un manejador `FSWatcher` y no ofrece un método para suscribirse a eventos.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/watch/example.md -->
